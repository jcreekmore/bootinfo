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

mod multiboot1 {
    use bytes::{Buf, IntoBuf};
    use std::fmt;
    pub const MAGIC: u32 = 0x1BADB002;

    bitflags! {
        pub struct Flags: u32 {
            const PAGE_ALIGNED_MODULES = 0x0000_0001;
            const REQUEST_MEMORY_MAP   = 0x0000_0002;
            const REQUEST_VIDEO_MODE   = 0x0000_0004;
            const ENTRY_ADDRS_VALID    = 0x0001_0000;
            const UNKNOWN_FLAGS        = 0xfffe_fff8;
        }
    }

    impl fmt::Display for Flags {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            let mut flags = vec![];

            if self.contains(Flags::PAGE_ALIGNED_MODULES) {
                flags.push("page-aligned-modules");
            }

            if self.contains(Flags::REQUEST_MEMORY_MAP) {
                flags.push("request-memory-map");
            }

            if self.contains(Flags::REQUEST_VIDEO_MODE) {
                flags.push("request-video-mode");
            }

            if self.contains(Flags::ENTRY_ADDRS_VALID) {
                flags.push("entry-addrs-valid");
            }

            write!(f, "[{}]", flags.join(", "))
        }
    }

    pub const GRAPHICS_MODE_TYPE_LINEAR: u32   = 0;
    pub const GRAPHICS_MODE_TYPE_EGA_TEXT: u32 = 1;

    pub const GRAPHICS_NO_PREFERENCE: u32 = 0;

    #[derive(Debug)]
    pub struct Header {
        magic: u32,
        flags: Flags,
        checksum: u32,
        header_addr: u32,
        load_addr: u32,
        load_end_addr: u32,
        bss_end_addr: u32,
        entry_addr: u32,
        mode_type: u32,
        width: u32,
        height: u32,
        depth: u32,
    }

    impl Header {
        pub fn parse(buf: ::bytes::Bytes) -> Option<Header> {
            let mut buf = buf.into_buf();
            while buf.remaining() > ::std::mem::size_of::<u32>() {
                let value = buf.get_u32::<::bytes::LittleEndian>();
                if value == MAGIC {
                    break;
                }
            }

            if buf.remaining() < (::std::mem::size_of::<u32>() * 11) {
                None
            } else {
                let flags = buf.get_u32::<::bytes::LittleEndian>();
                let checksum = buf.get_u32::<::bytes::LittleEndian>();
                if MAGIC.wrapping_add(flags).wrapping_add(checksum) != 0 {
                    return None;
                }

                Some(Header {
                    magic: MAGIC,
                    flags: Flags::from_bits_truncate(flags),
                    checksum: checksum,
                    header_addr: buf.get_u32::<::bytes::LittleEndian>(),
                    load_addr: buf.get_u32::<::bytes::LittleEndian>(),
                    load_end_addr: buf.get_u32::<::bytes::LittleEndian>(),
                    bss_end_addr: buf.get_u32::<::bytes::LittleEndian>(),
                    entry_addr: buf.get_u32::<::bytes::LittleEndian>(),
                    mode_type: buf.get_u32::<::bytes::LittleEndian>(),
                    width: buf.get_u32::<::bytes::LittleEndian>(),
                    height: buf.get_u32::<::bytes::LittleEndian>(),
                    depth: buf.get_u32::<::bytes::LittleEndian>(),
                })
            }
        }
    }

    impl fmt::Display for Header {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(f, "Multiboot Header\n")?;
            write!(f, "  Magic     : 0x{:08x}\n", self.magic)?;
            write!(f, "  Flags     : {} (0x{:08x})\n", self.flags, self.flags.bits())?;
            write!(f, "  Checksum  : 0x{:08x}\n", self.checksum)?;
            if self.flags.contains(Flags::ENTRY_ADDRS_VALID) {
                write!(f, "  Header    : 0x{:08x}\n", self.header_addr)?;
                write!(f, "  Load      : 0x{:08x}\n", self.load_addr)?;
                write!(f, "  Load End  : 0x{:08x}\n", self.load_end_addr)?;
                write!(f, "  BSS End   : 0x{:08x}\n", self.bss_end_addr)?;
            }
            if self.flags.contains(Flags::REQUEST_VIDEO_MODE) {
                let mode = match self.mode_type {
                    GRAPHICS_MODE_TYPE_LINEAR => "linear",
                    GRAPHICS_MODE_TYPE_EGA_TEXT => "ega",
                    _ => "unknown",
                };

                let width = match self.width {
                    GRAPHICS_NO_PREFERENCE => "no preference".into(),
                    x => format!("{}", x),
                };

                let height = match self.height {
                    GRAPHICS_NO_PREFERENCE => "no preference".into(),
                    x => format!("{}", x),
                };

                let depth = match self.depth {
                    GRAPHICS_NO_PREFERENCE => "no preference".into(),
                    x => format!("{}", x),
                };

                write!(f, "  Mode      : {} ({})\n", mode, self.mode_type)?;
                write!(f, "  Width     : {}\n", width)?;
                write!(f, "  Height    : {}\n", height)?;
                write!(f, "  Depth     : {}\n", depth)?;
            }
            Ok(())
        }
    }
}

fn create_buffer<R: Read>(rdr: R) -> Result<bytes::Bytes> {
    const BUFLEN: usize = (32 * 1024);
    let mut fp = rdr.take(BUFLEN as u64);

    let buffer = bytes::BytesMut::with_capacity(BUFLEN);
    let mut buffer = buffer.writer();

    io::copy(&mut fp, &mut buffer).chain_err(|| "failed to fill buffer with contents of input file")?;
    Ok(buffer.into_inner().freeze())
}

quick_main!{|| -> Result<()> {
    let opts = Opts::from_args();
    let fp = File::open(&opts.input).chain_err(|| format!("failed to open input file {}", &opts.input))?;

    let fp = flate2::read::GzDecoder::new(fp);
    let bytes = if fp.header().is_some() { create_buffer(fp) } else {
        let mut fp = fp.into_inner();
        fp.seek(io::SeekFrom::Start(0)).chain_err(|| "failed to seek back to beginning of file")?;
        create_buffer(fp)
    };

    let header = multiboot1::Header::parse(bytes?);
    if let Some(header) = header {
        println!("{}", header);
    }

    Ok(())
}}

