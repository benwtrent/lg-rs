use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter, Display};
use regex;
use std::cmp::{Ordering};
use std::borrow::Borrow;

#[derive(Eq, PartialEq, Hash)]
pub enum Token {
    WildCard,
    Val(String)
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Val(s) => write!(f, "{}", s.as_str()),
            Token::WildCard => write!(f, "{}", "<*>")
        }
    }
}

impl std::clone::Clone for Token {
    fn clone(&self) -> Self {
        match self {
            Token::WildCard => Token::WildCard,
            Token::Val(s) => Token::Val(s.clone())
        }
    }
}

#[derive(PartialEq)]
struct GroupSimilarity {
    approximate_similarity: f32,
    exact_similarity: f32,
}

impl core::cmp::PartialOrd for GroupSimilarity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.exact_similarity.partial_cmp(&other.exact_similarity) {
            Some(order) => {
                match order {
                    Ordering::Equal => {
                        self.approximate_similarity.partial_cmp(&other.approximate_similarity)
                    },
                    Ordering::Less => Some(Ordering::Less),
                    Ordering::Greater => Some(Ordering::Greater)
                }
            },
            None => None
        }
    }
}

pub struct DrainTreeLogGroup {
    log_tokens: Vec<Token>,
    num_matched: u64,
}

impl fmt::Display for DrainTreeLogGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}, count [{}] ", self.log_tokens.iter().map(|t| t.to_string()).collect::<Vec<String>>().join(" "), self.num_matched)
    }
}

impl DrainTreeLogGroup {

    pub fn new(log_tokens: Vec<Token>) -> DrainTreeLogGroup {
        DrainTreeLogGroup {
            log_tokens,
            num_matched: 1
        }
    }

    fn similarity(&self, log: &[Token]) -> GroupSimilarity {
        let len = self.log_tokens.len();
        let mut approximate_similarity: usize = 0;
        let mut exact_similarity: usize = 0;

        for (pattern, token) in self.log_tokens.iter().zip(log.iter()) {
            if token == pattern {
                approximate_similarity += 1;
                exact_similarity += 1;
            } else if *pattern == Token::WildCard {
                approximate_similarity += 1;
            }
        }
        GroupSimilarity {
            approximate_similarity: (approximate_similarity / len) as f32,
            exact_similarity: (exact_similarity / len) as f32
        }
    }

    pub fn add_log(&mut self, log: &[Token]) {
        for i in 0..log.len() {
            let token = &self.log_tokens[i];
            if token != &Token::WildCard {
                let other_token = &log[i];
                if token != other_token {
                    self.log_tokens[i] = Token::WildCard;
                }
            }
        }
        self.num_matched += 1;
    }

}

struct LeafNode {
    log_groups: Vec<DrainTreeLogGroup>
}

struct GroupAndSimilarity {
    group_index: usize,
    similarity: GroupSimilarity,
}

impl LeafNode {

    fn best_group(&self, log_tokens: &[Token]) -> Option<GroupAndSimilarity> {
        let mut max_similarity = match self.log_groups.get(0) {
            Some(group) => group.similarity(log_tokens),
            None => return None
        };
        let mut group_index:usize = 0;
        for i in 1..self.log_groups.len() {
            let group = self.log_groups.get(i).unwrap();
            let similarity = group.similarity(log_tokens);
            if similarity > max_similarity {
                max_similarity = similarity;
                group_index = i;
            }
        }
        Some(GroupAndSimilarity {
            group_index,
            similarity: max_similarity
        })
    }

    fn add_to_group(&mut self,
                    group: Option<GroupAndSimilarity>,
                    min_similarity: &f32,
                    log_tokens: &[Token]) {
        match group {
            Some(gas) => {
                if gas.similarity.approximate_similarity < *min_similarity {
                    self.log_groups.push(DrainTreeLogGroup::new(log_tokens.to_vec()))
                } else {
                    self.log_groups
                        .get_mut(gas.group_index)
                        .expect(format!("bad log group index [{}]", gas.group_index).as_str())
                        .add_log(log_tokens)
                }
            },
            None => {
                self.log_groups.push(DrainTreeLogGroup::new(log_tokens.to_vec()))
            }
        }
    }

}

struct InnerNode {
    children: HashMap<Token, DrainTreeNode>,
    depth: usize
}

enum DrainTreeNode {
    InnerNode(InnerNode),
    LeafNode(LeafNode)
}

impl Display for DrainTreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut str = String::new();
        match self {
            DrainTreeNode::InnerNode(node) => {
                for (k, v) in node.children.iter() {
                    str += &format!("{}Token: {} -> Children [{}]\n", " ".repeat(node.depth), k, v).to_string();
                }
            },
            DrainTreeNode::LeafNode(node) => {
                for lg in node.log_groups.iter() {
                    str += &format!("group [{}]", lg).to_string();
                }

            }
        }
        write!(f, "[\n{}\n]", str)
    }
}

impl DrainTreeNode {

    fn log_groups(&self) -> Vec<&DrainTreeLogGroup> {
        match self {
            DrainTreeNode::LeafNode(leaf) =>
                leaf.log_groups
                    .iter().map(|n| n.borrow())
                    .collect::<Vec<&DrainTreeLogGroup>>(),
            DrainTreeNode::InnerNode(inner) =>
                inner.children
                    .values()
                    .flat_map(|n| n.log_groups())
                    .collect::<Vec<&DrainTreeLogGroup>>()
        }
    }

    fn inner(depth: usize) -> DrainTreeNode {
        DrainTreeNode::InnerNode(InnerNode{
            children: HashMap::new(),
            depth
        })
    }

    fn leaf() -> DrainTreeNode {
        DrainTreeNode::LeafNode(LeafNode{
            log_groups: vec![]
        })
    }

    fn add_child_recur(&mut self,
                       depth: usize,
                       max_depth: &u16,
                       max_children: &u16,
                       min_similarity: &f32,
                       log_tokens: &[Token]) {
        let next = depth + 1;
        let token = match &log_tokens[next] {
            Token::Val(s) => {
                if s.chars().any(|c| c.is_numeric()) {
                    Token::WildCard
                } else {
                    Token::Val(s.clone())
                }
            }
            Token::WildCard => Token::WildCard
        };
        if next == log_tokens.len() - 1 || next == *max_depth as usize {
            if let DrainTreeNode::InnerNode(node) = self {
                let child = node.children
                    .entry(token)
                    .or_insert(DrainTreeNode::leaf());
                if let DrainTreeNode::LeafNode(leaf) = child {
                    let best_group = leaf.best_group(log_tokens);
                    leaf.add_to_group(best_group, min_similarity, log_tokens);
                }
            }
            return;
        }
        match self {
            DrainTreeNode::InnerNode(inner) => {
                let child = if !inner.children.contains_key(&token)
                    && inner.children.len() > *max_children as usize {
                    inner.children
                        .entry(Token::WildCard)
                        .or_insert(DrainTreeNode::inner(next))
                } else {
                    inner.children
                        .entry(token)
                        .or_insert(DrainTreeNode::inner(next))
                };
                child.add_child_recur(next, max_depth, max_children, min_similarity, log_tokens);
            },
            DrainTreeNode::LeafNode(leaf) => {
                let best_group = leaf.best_group(log_tokens);
                leaf.add_to_group(best_group, min_similarity, log_tokens);
            },
        }
    }
}

pub struct DrainTree {
    root: HashMap<usize, DrainTreeNode>,
    max_depth: u16,
    max_children: u16,
    min_similarity: f32,
    filter_patterns: Vec<regex::Regex>,
}

impl Display for DrainTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut str = String::new();
        for (k, v) in self.root.iter() {
            str += &format!("Len: {} -> [ {} ]\n", k, v).to_string();
        }
        write!(f, "[\n{}\n]", str)
    }
}

impl DrainTree {

    pub fn new() -> Self {
        DrainTree {
            root: HashMap::new(),
            filter_patterns: vec![],
            max_depth: 5,
            max_children: 100,
            min_similarity: 0.5
        }
    }

    pub fn max_depth(mut self, max_depth: u16) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn max_children(mut self, max_children: u16) -> Self {
        self.max_children = max_children;
        self
    }

    pub fn min_similarity(mut self, min_similarity: f32) -> Self {
        self.min_similarity = min_similarity;
        self
    }

    pub fn filter_patterns(mut self, filter_patterns: Vec<regex::Regex>) -> Self {
        self.filter_patterns = filter_patterns;
        self
    }

    fn process(filter_patterns: &Vec<regex::Regex>, log_line: String) -> Vec<Token> {
        log_line.split(' ')
            .map(|t|t.trim())
            .map(|t|
                if filter_patterns
                    .iter()
                    .any(|p| p.is_match(t)) {
                    Token::WildCard
                } else {
                    Token::Val(String::from(t))
                })
            .collect()
    }

    fn dig_inner_prefix_tree<'a>(&self,
                             child: &'a DrainTreeNode,
                             processed_log: &[Token]) -> Option<&'a DrainTreeLogGroup> {
        let mut current_node = child;
        for t in processed_log.iter() {
            match current_node {
                DrainTreeNode::LeafNode(leaf) => {
                    return match leaf.best_group(processed_log) {
                        Some(gas) => {
                            Some(&leaf.log_groups[gas.group_index])
                        },
                        None => return None
                    };
                },
                DrainTreeNode::InnerNode(node) => {
                    match node.children.get(t) {
                        Some(n) => current_node = n,
                        None => return None
                    }
                }
            }
        }
        return None;
    }

    fn log_group_for_tokens(&self, processed_log: &[Token]) -> Option<& DrainTreeLogGroup> {
        match self.root.get(&processed_log.len()) {
            Some(node) => self.dig_inner_prefix_tree(node, processed_log),
            None => Option::None
        }
    }

    pub fn log_group(&self, log: String) -> Option<&DrainTreeLogGroup> {
        let tokens = DrainTree::process(&self.filter_patterns, log);
        self.log_group_for_tokens(tokens.as_slice())
    }

    pub fn add_log_line(&mut self, log_line: String) {
        let tokens = DrainTree::process(&self.filter_patterns, log_line);
        let len = tokens.len();
        self.root
            .entry(len)
            .or_insert(DrainTreeNode::inner(0))
            .add_child_recur(0, &self.max_depth, &self.max_children, &self.min_similarity, tokens.as_slice());
    }

    pub fn log_groups(&self) -> Vec<&DrainTreeLogGroup> {
        self.root.values().flat_map(|n| n.log_groups()).collect::<Vec<&DrainTreeLogGroup>>()
    }
}
