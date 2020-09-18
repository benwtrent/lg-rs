mod drain;

use std::env;
use std::fs;
use std::io::{self, BufReader, BufRead};
use crate::drain::{DrainTree};
use regex;
use clap;
use grok;

/*fn reader(mut args: env::Args) -> Box<BufReader<impl Read>> {
    args.next();

    return match args.next() {
        Some(file) => Box::new(BufReader::new(OpenOptions::new()
            .read(true)
            .open(file)
            .expect("unable to open file"))),
        None => {
            let stdin = std::io::stdin();
            Box::new(BufReader::new(stdin.lock()))
        },
    };
}*/

fn main() {

    let filter_patterns = vec![
        regex::Regex::new(r"blk_(|-)[0-9]+").expect("bad pattern"), //blockid
        regex::Regex::new(r"(/|)([0-9]+\.){3}[0-9]+(:[0-9]+|)(:|)").expect("bad pattern"), //IP
        regex::Regex::new(r"([^A-Za-z0-9])(\-?\+?\d+)([^A-Za-z0-9])|[0-9]+$").expect("bad pattern"), //Num
    ];
    let mut drain = DrainTree::new()
        .filter_patterns(filter_patterns)
        .max_depth(4)
        .max_children(100)
        .min_similarity(0.5);
    let input = env::args().nth(1);
    let reader: Box<dyn BufRead> = match input {
        None => Box::new(BufReader::new(io::stdin())),
        Some(filename) => Box::new(BufReader::new(fs::File::open(filename).unwrap()))
    };
    for line in reader.lines() {
        if let Ok(s) = line {
            let l = s.split(" ")
            .skip(5)
            .collect::<Vec<&str>>()
            .join(" ");
            drain.add_log_line(l);
        }
    }
    drain.log_groups().iter().for_each(|f| println!("{}", *f));
}
