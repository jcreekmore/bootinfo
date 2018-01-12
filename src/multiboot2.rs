use bytes::{self, Buf, IntoBuf};
use std::fmt;
use std::mem::size_of;
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
    tags: Vec<Tag>,
}

const TAG_INFORMATION_REQUEST: u16 = 1;
const TAG_ADDRESS: u16 = 2;
const TAG_ENTRY_ADDRESS: u16 = 3;
const TAG_FLAGS: u16 = 4;
const TAG_FRAMEBUFFER: u16 = 5;
const TAG_MODULE_ALIGNMENT: u16 = 6;
const TAG_EFI_BOOT_SERVICES: u16 = 7;
const TAG_EFI_I386_ENTRY_ADDRESS: u16 = 8;
const TAG_EFI_AMD64_ENTRY_ADDRESS: u16 = 9;
const TAG_RELOCATABLE: u16 = 10;

#[derive(Debug)]
pub enum TagVariant {
    InformationRequest { mbi_tag_types: Vec<u16> },
    Address { header_addr: u32, load_addr: u32, load_end_addr: u32, bss_end_addr: u32 },
    Flags { console_flags: u32 },
    Framebuffer { width: u32, height: u32, depth: u32 },
    EfiBootServices,
    ModuleAlignment,
    Unknown,
}

#[derive(Debug)]
pub struct Tag {
    typ: u16,
    flags: u16,
    size: u32,
    variant: TagVariant,
}

impl Header {
    pub fn parse(buf: bytes::Bytes) -> Option<Header> {
        let mut buf = buf.into_buf();
        while buf.remaining() > size_of::<u32>() {
            let value = buf.get_u32::<bytes::LittleEndian>();
            if value == MAGIC {
                break;
            }
        }

        if buf.remaining() < (size_of::<u32>() * 3) {
            return None;
        }

        let architecture = buf.get_u32::<bytes::LittleEndian>();
        let header_length = buf.get_u32::<bytes::LittleEndian>();
        let checksum = buf.get_u32::<bytes::LittleEndian>();
        if MAGIC.wrapping_add(architecture).wrapping_add(header_length).wrapping_add(checksum) != 0 {
            return None;
        }

        if buf.remaining() < (header_length as usize - (size_of::<u32>() * 4)) {
            return None;
        }

        let mut typ = 1;
        let mut flags = 0;
        let mut size = 0;
        let mut tags = vec![];

        while typ != 0 && size != 8 {
            typ = buf.get_u16::<bytes::LittleEndian>();
            flags = buf.get_u16::<bytes::LittleEndian>();
            size = buf.get_u32::<bytes::LittleEndian>();

            let read_more = size as usize - (size_of::<u32>() * 2);

            let variant = match typ {
                TAG_INFORMATION_REQUEST => {
                    TagVariant::InformationRequest { mbi_tag_types: vec![] }
                },
                TAG_ADDRESS => { TagVariant::Unknown },
                TAG_ENTRY_ADDRESS => { TagVariant::Unknown },
                TAG_FLAGS => { TagVariant::Unknown },
                TAG_FRAMEBUFFER => { TagVariant::Unknown },
                TAG_MODULE_ALIGNMENT => { TagVariant::Unknown },
                TAG_EFI_BOOT_SERVICES => { TagVariant::Unknown },
                TAG_EFI_I386_ENTRY_ADDRESS => { TagVariant::Unknown },
                TAG_EFI_AMD64_ENTRY_ADDRESS => { TagVariant::Unknown },
                TAG_RELOCATABLE => { TagVariant::Unknown },
                _ => { TagVariant::Unknown },
            };

            tags.push(Tag { typ: typ, flags: flags, size: size, variant: variant });

            buf.advance(read_more);
        }

        Some(Header {
                 magic: MAGIC,
                 architecture: architecture,
                 header_length: header_length,
                 checksum: checksum,
                 tags: tags,
             })
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Multiboot2 Header\n")?;
        write!(f, "  Magic       : 0x{:08x}\n", self.magic)?;
        write!(f, "  Arch        : 0x{:08x}\n", self.architecture)?;
        write!(f, "  Header Len  : 0x{:08x}\n", self.header_length)?;
        write!(f, "  Checksum    : 0x{:08x}\n", self.checksum)?;
        for x in &self.tags {
            write!(f, "  Tag: {}\n", x.typ);
            write!(f, "    Flags    : 0x{:04x}\n", x.flags)?;
            write!(f, "    Size     : 0x{:08x}\n", x.size)?;
        }
        Ok(())
    }
}
