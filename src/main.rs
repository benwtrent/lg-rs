use clap::{App, Arg};
use drain_rs::DrainTree;
use grok;
use serde_json;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};

fn verify_positive_short(s: String) -> Result<(), String> {
    match s.trim().parse::<u32>() {
        Ok(num) => {
            if num > 0 {
                Ok(())
            } else {
                return Result::Err(String::from("must be positive value"));
            }
        }
        Err(f) => return Result::Err(f.to_string()),
    }
}

fn verify_float_between(min: f32, max: f32) -> Box<dyn Fn(String) -> Result<(), String>> {
    Box::new(move |s| match s.trim().parse::<f32>() {
        Ok(num) => {
            if num >= min && num <= max {
                Ok(())
            } else {
                Result::Err(String::from(format!(
                    "must be between [{}] and [{}] (inclusive)",
                    min, max
                )))
            }
        }
        Err(f) => Result::Err(f.to_string()),
    })
}

fn main() {
    let mut g = grok::Grok::with_patterns();
    let matches = App::new("log groups (lg)")
        .version("0.1.0")
        .author("Ben Trent <ben.w.trent@gmail.com>")
        .about("Automatically categorizes and groups semi-structured text")
        .after_help("Example: lg-rs HDFS_2k.log --log-pattern=\"%{NUMBER:date} %{NUMBER:time} %{NUMBER:proc} %{LOGLEVEL:level} %{DATA:component}: %{GREEDYDATA:content}\" --filter-patterns=\"blk_(|-)[0-9]+,%{IPV4:ip_address},%{NUMBER:number}\"")
        .arg(
            Arg::with_name("INPUT_FILE")
                .index(1)
                .required(false)
        )
        .arg(
            Arg::with_name("MAX_DEPTH")
                .long("max-depth")
                .validator(verify_positive_short)
                .default_value("4")
                .value_name("NUM")
                .help("the maximum match tree depth")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("MAX_CHILDREN")
                .long("max-children")
                .value_name("NUM")
                .validator(verify_positive_short)
                .default_value("100")
                .help("how many immediate children should each prefix-tree node have")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("MIN_SIMILARITY")
                .required(false)
                .validator(verify_float_between(0.0, 1.0))
                .long("min-similarity")
                .takes_value(true)
                .default_value("0.4")
        )
        .arg(
            Arg::with_name("LOG_PATTERN")
                .long("log-pattern")
                .required(false)
                .takes_value(true)
                .help("Provide the overall GROK log pattern for the text input. If provided, you might want to update --group-field to match the semi-structured text field that is returned by the log-pattern"),
        ).arg(
        Arg::with_name("GROUP_FIELD")
            .long("group-field")
            .default_value("content")
            .takes_value(true)
            .required(false)
        .help("The extracted field on which to group. Only used if --log-pattern is provided. Defaults to [content]")
    ).arg(
        Arg::with_name("FILTER_PATTERNS")
            .long("filter-patterns")
            .takes_value(true)
            .required(false)
            .multiple(true)
            .use_delimiter(true)
        .help("The GROK filter patterns to apply when grouping logs. This provides a way to extract out known variable patterns (e.g. ip addresses)"),
    ).arg(
        Arg::with_name("FROM_MODEL")
            .long("from-model")
            .takes_value(true)
            .required(false)
            .help("The previously dumped model from which to take ALL parameters and initialize the log groupings."),
    ).arg(
        Arg::with_name("OUTPUT_MODEL_FILE")
            .short("o")
            .long("output-model")
            .takes_value(true)
            .required(false)
            .value_name("FILE")
            .help("The file where to dump the resulting log grouping tree and settings. This allows the built prefix tree to be shared and reused."),
    )
        .get_matches();

    let max_depth = matches
        .value_of("MAX_DEPTH")
        .unwrap_or("4")
        .trim()
        .parse::<u16>()
        .expect("invalid value for max-depth");
    let max_children = matches
        .value_of("MAX_CHILDREN")
        .unwrap_or("100")
        .trim()
        .parse::<u16>()
        .expect("invalid value for max-children");
    let min_similarity = matches
        .value_of("MIN_SIMILARITY")
        .unwrap_or("0.4")
        .trim()
        .parse::<f32>()
        .expect("invalid value for min-similarity");
    let drain = match matches.value_of("FROM_MODEL") {
        Some(path) => {
            let file = fs::File::open(path).expect("unable to find file");
            let reader = BufReader::new(file);
            // Read the JSON contents of the file as an instance of `User`.
            serde_json::from_reader(reader).expect("failed to build json reader")
        }
        None => {
            let drain = DrainTree::new()
                .max_depth(max_depth)
                .max_children(max_children)
                .min_similarity(min_similarity);
            let drain = match matches.value_of("LOG_PATTERN") {
                Some(val) => {
                    let group_field = matches.value_of("GROUP_FIELD").unwrap_or("content");
                    drain.log_pattern(val, group_field)
                }
                _ => drain,
            };

            match matches.values_of("FILTER_PATTERNS") {
                Some(val) => drain.filter_patterns(val.collect::<Vec<&str>>()),
                _ => drain,
            }
        }
    };

    let mut drain = drain.build_patterns(&mut g);
    let input = matches.value_of("INPUT_FILE");
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
    if let Some(output) = matches.value_of("OUTPUT_MODEL_FILE") {
        let dump = serde_json::to_string(&drain).expect("failed to serialize");
        let mut file = fs::File::create(output).expect("unable to open file");
        file.write_all(dump.as_bytes()).expect("unable to write");
    }
}
