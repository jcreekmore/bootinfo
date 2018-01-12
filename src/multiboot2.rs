use bytes::{self, Buf, IntoBuf};
use std::fmt;
pub const MAGIC: u32 = 0xE85250D6;

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

#[derive(Debug)]
pub struct Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
}

impl Header {
    pub fn parse(buf: bytes::Bytes) -> Option<Header> {
        let mut buf = buf.into_buf();
        while buf.remaining() > ::std::mem::size_of::<u32>() {
            let value = buf.get_u32::<bytes::LittleEndian>();
            if value == MAGIC {
                break;
            }
        }

        if buf.remaining() < (::std::mem::size_of::<u32>() * 11) {
            None
        } else {
            let architecture = buf.get_u32::<bytes::LittleEndian>();
            let header_length = buf.get_u32::<bytes::LittleEndian>();
            let checksum = buf.get_u32::<bytes::LittleEndian>();
            if MAGIC.wrapping_add(architecture).wrapping_add(header_length).wrapping_add(checksum) != 0 {
                return None;
            }

            Some(Header {
                     magic: MAGIC,
                     architecture: architecture,
                     header_length: header_length,
                     checksum: checksum,
                 })
        }
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Multiboot2 Header\n")?;
        write!(f, "  Magic       : 0x{:08x}\n", self.magic)?;
        write!(f, "  Arch        : 0x{:08x}\n", self.architecture)?;
        write!(f, "  Header Len  : 0x{:08x}\n", self.header_length)?;
        write!(f, "  Checksum    : 0x{:08x}\n", self.checksum)?;
        Ok(())
    }
}
