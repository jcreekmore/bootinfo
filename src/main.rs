#[macro_use]
extern crate bitflags;
extern crate bytes;
#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate error_chain;
extern crate flate2;
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

fn create_buffer<R: Read>(rdr: R, buflen: usize) -> Result<bytes::Bytes> {
    let mut fp = rdr.take(buflen as u64);

    let buffer = bytes::BytesMut::with_capacity(buflen);
    let mut buffer = buffer.writer();

    io::copy(&mut fp, &mut buffer)
        .chain_err(|| "failed to fill buffer with contents of input file")?;
    Ok(buffer.into_inner().freeze())
}

quick_main!{|| -> Result<()> {
    let descriptors = register_descriptors();

    let opts = Opts::from_args();
    let fp = File::open(&opts.input)
        .chain_err(|| format!("failed to open input file {}", &opts.input))?;

    // Grab the maximum range that the header can be found
    let max_range = &descriptors.iter().map(|d| d.max_range).max().unwrap_or(0);

    let fp = flate2::read::GzDecoder::new(fp);
    let bytes = if fp.header().is_some() { create_buffer(fp, *max_range) } else {
        let mut fp = fp.into_inner();
        fp.seek(io::SeekFrom::Start(0)).chain_err(|| "failed to seek back to beginning of file")?;
        create_buffer(fp, *max_range)
    };

    let bytes = bytes?;

    for info in &descriptors {
        let header = info.parse(bytes.clone());
        if let Some(header) = header {
            println!("{}", header);
        }
    }

    Ok(())
}}
