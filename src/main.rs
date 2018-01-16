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
use std::io::{self, Read, Seek};
use std::fs::File;

pub trait BootInfo: std::fmt::Display {}
pub type ParseBootInfo = fn(bytes::Bytes) -> Option<Box<BootInfo>>;

pub struct Descriptor {
    pub name: &'static str,
    pub max_range: usize,
    parser: ParseBootInfo,
}

impl Descriptor {
    pub fn parse(&self, buf: bytes::Bytes) -> Option<Box<BootInfo>> {
        (self.parser)(buf)
    }
}

mod multiboot1;
mod multiboot2;

fn register_descriptors() -> Vec<Descriptor> {
    let mut descs = vec![];
    multiboot1::register(&mut descs);
    multiboot2::register(&mut descs);
    descs
}

lazy_static! {
    static ref INFO: Vec<Descriptor> = register_descriptors();
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

quick_main!{|| -> Result<()> {
    // Parse the arguments
    let opts = Opts::from_args();

    // Grab the maximum range that the header can be found
    let max_range = INFO.iter().map(|d| d.max_range).max().unwrap_or(0);

    // Get the possible header bytes out of the file
    let bytes = possible_header_bytes(&opts.input, max_range)?;

    // For each known descriptor
    for info in INFO.iter() {
        // Attempt to parse the possible header bytes as that type
        let header = info.parse(bytes.clone());
        // If it is the correct type
        if let Some(header) = header {
            // Print the header fields out
            println!("{}", header);
        }
    }

    Ok(())
}}
