mod drain;

use crate::drain::DrainTree;
use grok;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};

fn main() {
    let mut g = grok::Grok::with_patterns();

    let filter_patterns = vec![
        "blk_(|-)[0-9]+",     //blockid
        "%{IPV4:ip_address}", //IP
        "%{NUMBER:number}",   //Num
    ];
    let mut drain = DrainTree::new()
        .filter_patterns(filter_patterns)
        .max_depth(4)
        .max_children(100)
        .min_similarity(0.5)
        .log_pattern("%{NUMBER:date} %{NUMBER:time} %{NUMBER:proc} %{LOGLEVEL:level} %{DATA:component}: %{GREEDYDATA:content}", "content")
        .build_patterns(&mut g);
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
