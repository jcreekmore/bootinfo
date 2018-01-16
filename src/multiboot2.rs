use bytes::{self, Buf, IntoBuf};
use std::fmt;
use std::mem::size_of;
pub const MAGIC: u32 = 0xE85250D6;

bitflags! {
        pub struct Flags: u16 {
            const OPTIONAL      = 0x0001;
            const UNKNOWN_FLAGS = 0xfffe;
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

const TAG_ENDING: u16 = 0;
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
    InformationRequest { mbi_tag_types: Vec<u32> },
    Address {
        header_addr: u32,
        load_addr: u32,
        load_end_addr: u32,
        bss_end_addr: u32,
    },
    Entry { entry_addr: u32 },
    EfiI386Entry { entry_addr: u32 },
    EfiAmd64Entry { entry_addr: u32 },
    Flags { console_flags: u32 },
    Framebuffer { width: u32, height: u32, depth: u32 },
    EfiBootServices,
    ModuleAlignment,
    Relocatable {
        min_addr: u32,
        max_addr: u32,
        align: u32,
        preference: u32,
    },
    Unknown,
}

#[derive(Debug)]
pub struct Tag {
    typ: u16,
    flags: Flags,
    size: u32,
    variant: TagVariant,
}

fn ending_tag(typ: u16, size: u32) -> bool {
    (typ == TAG_ENDING && size == 8)
}

impl super::BootInfo for Header {}

impl Header {
    pub fn parse(buf: bytes::Bytes) -> Option<Box<super::BootInfo>> {
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
        if MAGIC.wrapping_add(architecture)
            .wrapping_add(header_length)
            .wrapping_add(checksum) != 0 {
            return None;
        }

        if buf.remaining() < (header_length as usize - (size_of::<u32>() * 4)) {
            return None;
        }

        let mut typ = buf.get_u16::<bytes::LittleEndian>();
        let mut flags = buf.get_u16::<bytes::LittleEndian>();
        let mut size = buf.get_u32::<bytes::LittleEndian>();
        let mut tags = vec![];

        while !ending_tag(typ, size) {
            let read_more = size as usize - (size_of::<u32>() * 2);
            let padding = if read_more % 8 != 0 {
                ((read_more + 8) % 8)
            } else {
                0
            };

            let variant = match typ {
                TAG_INFORMATION_REQUEST => {
                    let mut info = vec![];
                    for _ in 0..(read_more / size_of::<u32>()) {
                        info.push(buf.get_u32::<bytes::LittleEndian>());
                    }
                    TagVariant::InformationRequest { mbi_tag_types: info }
                }
                TAG_ADDRESS => {
                    let header = buf.get_u32::<bytes::LittleEndian>();
                    let load = buf.get_u32::<bytes::LittleEndian>();
                    let load_end = buf.get_u32::<bytes::LittleEndian>();
                    let bss_end = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::Address {
                        header_addr: header,
                        load_addr: load,
                        load_end_addr: load_end,
                        bss_end_addr: bss_end,
                    }
                }
                TAG_ENTRY_ADDRESS => {
                    let entry = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::Entry { entry_addr: entry }
                }
                TAG_FLAGS => {
                    let flags = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::Flags { console_flags: flags }
                }
                TAG_FRAMEBUFFER => {
                    let width = buf.get_u32::<bytes::LittleEndian>();
                    let height = buf.get_u32::<bytes::LittleEndian>();
                    let depth = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::Framebuffer {
                        width: width,
                        height: height,
                        depth: depth,
                    }
                }
                TAG_MODULE_ALIGNMENT => TagVariant::ModuleAlignment,
                TAG_EFI_BOOT_SERVICES => TagVariant::EfiBootServices,
                TAG_EFI_I386_ENTRY_ADDRESS => {
                    let entry = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::EfiI386Entry { entry_addr: entry }
                }
                TAG_EFI_AMD64_ENTRY_ADDRESS => {
                    let entry = buf.get_u32::<bytes::LittleEndian>();
                    TagVariant::EfiAmd64Entry { entry_addr: entry }
                }
                TAG_RELOCATABLE => {
                    let min_addr = buf.get_u32::<bytes::LittleEndian>();
                    let max_addr = buf.get_u32::<bytes::LittleEndian>();
                    let align = buf.get_u32::<bytes::LittleEndian>();
                    let preference = buf.get_u32::<bytes::LittleEndian>();

                    TagVariant::Relocatable {
                        min_addr: min_addr,
                        max_addr: max_addr,
                        align: align,
                        preference: preference,
                    }
                }
                _ => {
                    buf.advance(read_more);
                    TagVariant::Unknown
                }
            };
            tags.push(Tag {
                typ: typ,
                flags: Flags::from_bits_truncate(flags),
                size: size,
                variant: variant,
            });

            buf.advance(padding);

            typ = buf.get_u16::<bytes::LittleEndian>();
            flags = buf.get_u16::<bytes::LittleEndian>();
            size = buf.get_u32::<bytes::LittleEndian>();
        }

        let header = Header {
            magic: MAGIC,
            architecture: architecture,
            header_length: header_length,
            checksum: checksum,
            tags: tags,
        };
        Some(Box::new(header) as Box<super::BootInfo>)
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
            write!(f, "{}", x)?;
        }
        Ok(())
    }
}

impl fmt::Display for TagVariant {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match *self {
            TagVariant::InformationRequest { .. } => "Information Request",
            TagVariant::Address { .. } => "Address",
            TagVariant::Entry { .. } => "Entry",
            TagVariant::EfiI386Entry { .. } => "EFI i386 Entry",
            TagVariant::EfiAmd64Entry { .. } => "EFI amd64 Entry",
            TagVariant::Flags { .. } => "Flags",
            TagVariant::Framebuffer { .. } => "Framebuffer",
            TagVariant::EfiBootServices => "EFI Boot Services",
            TagVariant::ModuleAlignment => "Module Alignment",
            TagVariant::Relocatable { .. } => "Relocatable",
            TagVariant::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut flags = vec![];

        if self.contains(Flags::OPTIONAL) {
            flags.push("optional");
        } else {
            flags.push("required");
        }

        write!(f, "[{}]", flags.join(", "))
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "  Tag: {} ({})\n", self.variant, self.typ)?;
        write!(f,
               "    Flags      : {} (0x{:04x})\n",
               self.flags,
               self.flags.bits())?;
        write!(f, "    Size       : {} bytes\n", self.size)?;
        match self.variant {
            TagVariant::InformationRequest { ref mbi_tag_types } => {
                write!(f, "    Types      : {:?}\n", mbi_tag_types)?;
            }
            TagVariant::Address { header_addr, load_addr, load_end_addr, bss_end_addr } => {
                write!(f, "    Header     : 0x{:.08x}\n", header_addr)?;
                write!(f, "    Load       : 0x{:.08x}\n", load_addr)?;
                write!(f, "    Load End   : 0x{:.08x}\n", load_end_addr)?;
                write!(f, "    BSS End    : 0x{:.08x}\n", bss_end_addr)?;
            }
            TagVariant::Entry { entry_addr } |
            TagVariant::EfiI386Entry { entry_addr } |
            TagVariant::EfiAmd64Entry { entry_addr } => {
                write!(f, "    Entry      : 0x{:.08x}\n", entry_addr)?;
            }
            TagVariant::Flags { console_flags } => {
                write!(f, "    Console    : 0x{:.08x}\n", console_flags)?;
            }
            TagVariant::Framebuffer { width, height, depth } => {
                write!(f, "    Width      : {}\n", width)?;
                write!(f, "    Height     : {}\n", height)?;
                write!(f, "    Depth      : {}\n", depth)?;
            }
            TagVariant::Relocatable { min_addr, max_addr, align, preference } => {
                write!(f, "    Min Addr   : 0x{:.08x}\n", min_addr)?;
                write!(f, "    Max Addr   : 0x{:.08x}\n", max_addr)?;
                write!(f, "    Align      : 0x{:.08x}\n", align)?;
                write!(f,
                       "    Preference : {}\n",
                       match preference {
                           0 => "none",
                           1 => "minimum",
                           2 => "maximum",
                           _ => "unknown",
                       })?;
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn register(descs: &mut Vec<super::Descriptor>) {
    descs.push(super::Descriptor {
        name: "multiboot2",
        max_range: 32768,
        parser: Header::parse,
    })
}
