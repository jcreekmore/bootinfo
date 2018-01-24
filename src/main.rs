#[macro_use]
extern crate bitflags;
extern crate bytes;
extern crate clap;
#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate error_chain;
extern crate flate2;
#[macro_use]
extern crate lazy_static;

use clap::{App, Arg};
use std::collections::HashSet;
use std::fs::File;

mod utils;
mod parsers;

use utils::header_bytes;

lazy_static! {
    static ref INFO: Vec<parsers::Descriptor> = parsers::register();
}

#[derive(Debug, ErrorChain)]
pub enum ErrorKind {
    Msg(String),
}

quick_main!{|| -> Result<i32> {
    // Grab the maximum range that the header can be found
    let possible_parsers: Vec<&str> = INFO.iter().map(|d| d.name).collect();

    let matches = App::new("bootinfo")
                .about("Display boot information from a file")
                .arg(Arg::with_name("quiet")
                     .short("q")
                     .long("quiet")
                     .help("do not print the header information"))
                .arg(Arg::with_name("only")
                     .takes_value(true)
                     .multiple(true)
                     .value_delimiter(",")
                     .long("only")
                     .help("only look for specific boot info types")
                     .possible_values(&possible_parsers))
                .arg(Arg::with_name("INPUT")
                     .required(true)
                     .help("the input file to use"))
                .get_matches();

    let input = matches.value_of("INPUT").expect("INPUT is a required field");
    let quiet = matches.is_present("quiet");

    let allowed_parsers: Vec<&parsers::Descriptor> = match matches.values_of("only") {
        Some(only) => {
            let parsers: HashSet<_> = only.collect();
            INFO.iter().filter(|p| parsers.contains(p.name)).collect()
        },
        None => INFO.iter().collect()
    };

    // Grab the maximum range that the header can be found
    let max_range = allowed_parsers.iter().map(|d| d.max_range).max().unwrap_or(0);

    // Get the possible header bytes out of the file
    let fp = File::open(&input)
        .chain_err(|| format!("failed to open input file {}", input))?;

    let bytes = header_bytes(fp, max_range)?;

    // For each known descriptor
    let headers: Vec<Box<parsers::BootInfo>> = allowed_parsers.iter().filter_map(|info| {
        // Attempt to parse the possible header bytes as that type
        info.parse(bytes.clone())
    }).collect();

    // If we are not simply checking for presence
    if !quiet {
        for header in &headers {
            // Print the header fields out
            println!("{}", header);
        }
    }

    let status = if headers.is_empty() { 1 } else { 0 };

    Ok(status)
}}
