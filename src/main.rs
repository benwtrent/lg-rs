mod drain;

use crate::drain::DrainTree;
use grok;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};

fn main() {
    let mut g = grok::Grok::with_patterns();

    let filter_patterns = vec![
        g.compile("blk_(|-)[0-9]+", true).expect("bad pattern"), //blockid
        g.compile("%{IPV4:ip_address}", true).expect("bad pattern"), //IP
        g.compile("%{NUMBER:number}", true).expect("bad pattern"), //Num
    ];
    let mut drain = DrainTree::new()
        .filter_patterns(filter_patterns)
        .max_depth(4)
        .max_children(100)
        .min_similarity(0.5)
        .log_pattern(g.compile("%{NUMBER:date} %{NUMBER:time} %{NUMBER:proc} %{LOGLEVEL:level} %{DATA:component}: %{GREEDYDATA:content}", true).expect("bad pattern"), "content");
    let input = env::args().nth(1);
    let reader: Box<dyn BufRead> = match input {
        None => Box::new(BufReader::new(io::stdin())),
        Some(filename) => Box::new(BufReader::new(fs::File::open(filename).unwrap())),
    };
    for line in reader.lines() {
        if let Ok(s) = line {
            drain.add_log_line(s.as_str());
        }
    }
    drain.log_groups().iter().for_each(|f| println!("{}", *f));
}
