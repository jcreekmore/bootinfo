use bytes::{self, Buf, IntoBuf, LittleEndian};
use std::fmt;
use std::ffi::CString;
pub const MAGIC: u32 = 0x53726448;

#[derive(Debug)]
pub struct Header {
    setup_sects: u8,
    syssize: u32,
    header: u32,
    version_major: u8,
    version_minor: u8,
    realmode_swtch: Option<u32>,
    kernel_version: Option<CString>,
    load_flags: Option<u8>,
    code32_start: Option<u32>,
    initrd_addr_max: Option<u32>,
    kernel_alignment: Option<u32>,
    relocatable_kernel: Option<bool>,
    min_alignment: Option<u32>,
    xloadflags: Option<u16>,
    cmdline_size: Option<u32>,
    payload_offset: Option<u32>,
    payload_length: Option<u32>,
    pref_address: Option<u64>,
    init_size: Option<u32>,
    handover_offset: Option<u32>,
}

fn valid<V>(version: u16, allowed: (u8, u8), value: V) -> Option<V> {
    let major: u8 = (version >> 8) as u8;
    let minor: u8 = (version & 0xff) as u8;

    if major > allowed.0 || (major == allowed.0 && minor >= allowed.1) {
        Some(value)
    } else {
        None
    }
}

impl super::BootInfo for Header {}

impl Header {
    pub fn parse(buf: bytes::Bytes) -> Option<Box<super::BootInfo>> {
        let mut version_buf = buf.clone().into_buf();
        let mut buf = buf.into_buf();

        if buf.remaining() < 0x1f2 {
            return None;
        }

        buf.advance(0x1f1);
        let setup_sects = buf.get_u8();

        if buf.remaining() < ((setup_sects as usize * 512) - 0x1f2) {
            return None;
        }

        // move past deprecated root_flags
        buf.advance(2);
        let syssize = buf.get_u32::<LittleEndian>();

        // move past ram_size, vid_mode, root_dev, and boot_flag
        buf.advance(8);

        // move past jump
        buf.advance(2);
        let header = buf.get_u32::<LittleEndian>();

        // Explicitly not dealing with old boot protocols right now
        if header != MAGIC {
            return None;
        }

        let version = buf.get_u16::<LittleEndian>();
        let major = version >> 8;
        let minor = version & 0xff;

        // Require versions at least 2.0 or newer
        if major < 2 {
            return None;
        }

        let realmode_swtch = buf.get_u32::<LittleEndian>();
        // move past obsolete start_sys_seg
        buf.advance(2);
        let kernel_version = {
            let version = buf.get_u16::<LittleEndian>();
            if version != 0 && version < (0x200 * setup_sects as u16) {
                version_buf.advance(version as usize + 0x200);
                let s = unsafe {
                    CString::from_vec_unchecked(version_buf
                                                    .iter()
                                                    .take_while(|x| *x != 0)
                                                    .collect())
                };
                Some(s)
            } else {
                None
            }
        };

        // move past write-only type_of_loader
        buf.advance(1);
        let load_flags = buf.get_u8();
        // move past obsolete setup_move_size
        buf.advance(2);
        let code32_start = buf.get_u32::<LittleEndian>();
        // move past write-only ramdisk_image, ramdisk_size
        buf.advance(8);
        // move past obsolete bootsect_kludge
        buf.advance(4);
        // move past write-only heap_end_ptr, ext_loader_ver, ext_loader_type, cmdline_ptr
        buf.advance(8);

        let initrd_addr_max = buf.get_u32::<LittleEndian>();
        let kernel_alignment = buf.get_u32::<LittleEndian>();
        let relocatable_kernel = buf.get_u8() != 0;
        let min_alignment = 1 << buf.get_u8();
        let xloadflags = buf.get_u16::<LittleEndian>();
        let cmdline_size = buf.get_u32::<LittleEndian>();

        // move past write-only hardware_subarch
        buf.advance(4);

        let payload_offset = buf.get_u32::<LittleEndian>();
        let payload_length = buf.get_u32::<LittleEndian>();

        // move past write-only setup_data
        buf.advance(8);

        let pref_address = {
            let value = buf.get_u64::<LittleEndian>();
            if value != 0 { Some(value) } else { None }
        };
        let init_size = buf.get_u32::<LittleEndian>();
        let handover_offset = buf.get_u32::<LittleEndian>();

        let header = Header {
            setup_sects: setup_sects,
            syssize: syssize,
            header: header,
            version_major: major as u8,
            version_minor: minor as u8,
            realmode_swtch: valid(version, (2, 0), realmode_swtch),
            kernel_version: kernel_version,
            load_flags: valid(version, (2, 0), load_flags),
            code32_start: valid(version, (2, 0), code32_start),
            initrd_addr_max: valid(version, (2, 3), initrd_addr_max),
            kernel_alignment: valid(version, (2, 5), kernel_alignment),
            relocatable_kernel: valid(version, (2, 5), relocatable_kernel),
            min_alignment: valid(version, (2, 10), min_alignment),
            xloadflags: valid(version, (2, 12), xloadflags),
            cmdline_size: valid(version, (2, 6), cmdline_size),
            payload_offset: valid(version, (2, 8), payload_offset),
            payload_length: valid(version, (2, 8), payload_length),
            pref_address: pref_address.and_then(|addr| valid(version, (2, 10), addr)),
            init_size: valid(version, (2, 10), init_size),
            handover_offset: valid(version, (2, 11), handover_offset),
        };

        Some(Box::new(header) as Box<super::BootInfo>)
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Linux Boot Protocol\n")?;
        if let Some(ref version) = self.kernel_version {
            write!(f, "  Kernel Version: {}\n", version.to_string_lossy())?;
        }
        write!(f, "  Header: 0x{:.08x}\n", self.header)?;
        write!(f,
               "  Version: {}.{}\n",
               self.version_major,
               self.version_minor)?;
        write!(f,
               "  Setup Sectors: {}\n",
               if self.setup_sects == 0 {
                   4
               } else {
                   self.setup_sects
               })?;
        write!(f, "  PM Code Size: {} bytes\n", (self.syssize * 16))?;
        if let Some(realmode_swtch) = self.realmode_swtch {
            write!(f, "  Realmode Switch: 0x{:.08x}\n", realmode_swtch)?;
        }
        if let Some(load_flags) = self.load_flags {
            write!(f,
                   "  Loaded: {}\n",
                   if load_flags & 1 == 1 { "HIGH" } else { "LOW" })?;
        }
        if let Some(code32_start) = self.code32_start {
            write!(f, "  Code32 Start: 0x{:.08x}\n", code32_start)?;
        }
        if let Some(initrd_addr_max) = self.initrd_addr_max {
            write!(f, "  Initrd Addr Max: 0x{:.08x}\n", initrd_addr_max)?;
        }
        if let Some(kernel_alignment) = self.kernel_alignment {
            write!(f, "  Kernel Alignment: 0x{:.08x}\n", kernel_alignment)?;
        }
        if let Some(min_alignment) = self.min_alignment {
            write!(f, "  Min. Kernel Alignment: 0x{:.08x}\n", min_alignment)?;
        }
        if let Some(relocatable_kernel) = self.relocatable_kernel {
            write!(f, "  Relocatable?: {}\n", relocatable_kernel)?;
        }
        if let Some(xloadflags) = self.xloadflags {
            write!(f, "  xloadflags: 0x{:.04x}\n", xloadflags)?;
        }
        if let Some(cmdline_size) = self.cmdline_size {
            write!(f, "  Max Cmdline Size: {} bytes\n", cmdline_size)?;
        }
        if let Some(payload_offset) = self.payload_offset {
            write!(f, "  Payload Offset: 0x{:.08x}\n", payload_offset)?;
        }
        if let Some(payload_length) = self.payload_length {
            write!(f, "  Payload length: {} bytes\n", payload_length)?;
        }
        if let Some(init_size) = self.init_size {
            write!(f, "  init size: {} bytes\n", init_size)?;
        }
        if let Some(ref address) = self.pref_address {
            write!(f, "  Preferred load address: 0x{:.016x}\n", address)?;
        }
        if let Some(handover_offset) = self.handover_offset {
            write!(f, "  EFI Handover Offset: 0x{:.08x}\n", handover_offset)?;
        }
        Ok(())
    }
}

pub fn register(descs: &mut Vec<super::Descriptor>) {
    descs.push(super::Descriptor {
                   name: "linux",
                   max_range: 32768,
                   parser: Header::parse,
               })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use utils;
    const MULTIBOOT1: &[u8; 40000] = include_bytes!("../../test-data/multiboot1");
    const MULTIBOOT2: &[u8; 40000] = include_bytes!("../../test-data/multiboot2");
    const LINUXBOOT: &[u8; 40000] = include_bytes!("../../test-data/linuxboot");

    #[test]
    #[should_panic]
    fn parse_invalid_multiboot1() {
        let cursor = io::Cursor::new(MULTIBOOT1.as_ref());
        let bytes = utils::header_bytes(cursor, 8192).unwrap();
        Header::parse(bytes).unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_invalid_multiboot2() {
        let cursor = io::Cursor::new(MULTIBOOT2.as_ref());
        let bytes = utils::header_bytes(cursor, 32768).unwrap();
        Header::parse(bytes).unwrap();
    }

    #[test]
    fn parse_valid_linuxboot() {
        let cursor = io::Cursor::new(LINUXBOOT.as_ref());
        let bytes = utils::header_bytes(cursor, 32768).unwrap();
        Header::parse(bytes).unwrap();
    }
}
