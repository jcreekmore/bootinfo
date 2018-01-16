#[macro_use]
extern crate bitflags;
extern crate bytes;
#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate error_chain;
extern crate flate2;
#[macro_use]
extern crate lazy_static;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use bytes::BufMut;
use structopt::StructOpt;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read, Seek};

mod parsers;

lazy_static! {
    static ref INFO: Vec<parsers::Descriptor> = parsers::register();
}

#[derive(Debug, ErrorChain)]
pub enum ErrorKind {
    Msg(String),
}

#[derive(Debug, StructOpt)]
#[structopt(name = "bootinfo", about = "Display boot information found in a file")]
struct Opts {
    #[structopt(help = "Input file")]
    input: String,
    #[structopt(name = "PARSER", long = "is")]
    /// A single parser type such as multiboot1 or multiboot2
    is: Option<String>,
    #[structopt(name = "PARSERS", long = "print")]
    /// A comma-delimited list of parsers to probe for and print: multiboot1,multiboot2
    print: Option<String>,
}

/// Create a buffer from the file
fn create_buffer<R: Read>(rdr: R, buflen: usize) -> Result<bytes::Bytes> {
    let mut fp = rdr.take(buflen as u64);

    let buffer = bytes::BytesMut::with_capacity(buflen);
    let mut buffer = buffer.writer();

    io::copy(&mut fp, &mut buffer)
        .chain_err(|| "failed to fill buffer with contents of input file")?;
    Ok(buffer.into_inner().freeze())
}

/// Read out the possible header bytes from the file
fn possible_header_bytes(filename: &str, buflen: usize) -> Result<bytes::Bytes> {
    // Open the input file
    let fp = File::open(filename).chain_err(|| format!("failed to open input file {}", filename))?;

    // Assume that it is GZip-encoded
    let fp = flate2::read::GzDecoder::new(fp);
    // If it was in fact GZip-encoded
    if fp.header().is_some() {
        // Create a buffer out of the uncompressed bytes
        create_buffer(fp, buflen)
    } else {
        // Otherwise, we need to get back the original file
        let mut fp = fp.into_inner();
        // Rewind to the beginning of it
        fp.seek(io::SeekFrom::Start(0)).chain_err(|| "failed to seek back to beginning of file")?;
        // And create a buffer from the uncompressed bytes
        create_buffer(fp, buflen)
    }
}

quick_main!{|| -> Result<i32> {
    // Parse the arguments
    let opts = Opts::from_args();

    let allowed_parsers: Vec<&parsers::Descriptor> = match (opts.is.as_ref(), opts.print.as_ref()) {
        (Some(is), None) => {
            INFO.iter().filter(|p| p.name == is).collect()
        },
        (None, Some(print)) => {
            let parsers: HashSet<&str> = print.split(',').collect();
            INFO.iter().filter(|p| parsers.contains(p.name)).collect()
        },
        (None, None) => {
            INFO.iter().collect()
        },
        _ => {
            return Err("Can only specify one of --is and --print".into());
        }
    };

    // Grab the maximum range that the header can be found
    let max_range = allowed_parsers.iter().map(|d| d.max_range).max().unwrap_or(0);

    // Get the possible header bytes out of the file
    let bytes = possible_header_bytes(&opts.input, max_range)?;

    // For each known descriptor
    let headers: Vec<Box<parsers::BootInfo>> = allowed_parsers.iter().filter_map(|info| {
        // Attempt to parse the possible header bytes as that type
        info.parse(bytes.clone())
    }).collect();

    // If we are not simply checking for presence
    if opts.is.is_none() {
        for header in &headers {
            // Print the header fields out
            println!("{}", header);
        }
    }

    let status = if headers.is_empty() { 1 } else { 0 };

    Ok(status)
}}
