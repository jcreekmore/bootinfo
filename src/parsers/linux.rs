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
    realmode_swtch: u32,
    kernel_version: Option<CString>,
    load_flags: u8,
    code32_start: u32,
    initrd_addr_max: u32,
    kernel_alignment: u32,
    relocatable_kernel: bool,
    min_alignment: u32,
    xloadflags: u16,
    cmdline_size: u32,
    payload_offset: u32,
    payload_length: u32,
    pref_address: Option<u64>,
    init_size: u32,
    handover_offset: u32,
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

        // Explicitly not dealing with old boot protocols right now
        if major < 2 || minor < 12 {
            return None;
        }

        let realmode_swtch = buf.get_u32::<LittleEndian>();
        // move past obsolete start_sys_seg
        buf.advance(2);
        let kernel_version = {
            let version = buf.get_u16::<LittleEndian>();
            if version != 0 && version < (0x200 * setup_sects as u16) {
                version_buf.advance(version as usize + 0x200);
                let s = unsafe { CString::from_vec_unchecked(version_buf.iter().take_while(|x| *x != 0).collect()) };
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

        let header =  Header {
            setup_sects: setup_sects,
            syssize: syssize,
            header: header,
            version_major: major as u8,
            version_minor: minor as u8,
            realmode_swtch: realmode_swtch,
            kernel_version: kernel_version,
            load_flags: load_flags,
            code32_start: code32_start,
            initrd_addr_max: initrd_addr_max,
            kernel_alignment: kernel_alignment,
            relocatable_kernel: relocatable_kernel,
            min_alignment: min_alignment,
            xloadflags: xloadflags,
            cmdline_size: cmdline_size,
            payload_offset: payload_offset,
            payload_length: payload_length,
            pref_address: pref_address,
            init_size: init_size,
            handover_offset: handover_offset,
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
        write!(f, "  Version: {}.{}\n", self.version_major, self.version_minor)?;
        write!(f, "  Setup Sectors: {}\n", if self.setup_sects == 0 { 4 } else { self.setup_sects })?;
        write!(f, "  PM Code Size: {} bytes\n", (self.syssize * 16))?;
        write!(f, "  Realmode Switch: 0x{:.08x}\n", self.realmode_swtch)?;
        write!(f, "  Loaded: {}\n", if self.load_flags & 1 == 1 { "HIGH" } else { "LOW" })?;
        write!(f, "  Code32 Start: 0x{:.08x}\n", self.code32_start)?;
        write!(f, "  Initrd Addr Max: 0x{:.08x}\n", self.initrd_addr_max)?;
        write!(f, "  Kernel Alignment: 0x{:.08x}\n", self.kernel_alignment)?;
        write!(f, "  Min. Kernel Alignment: 0x{:.08x}\n", self.min_alignment)?;
        write!(f, "  Relocatable?: {}\n", self.relocatable_kernel)?;
        write!(f, "  xloadflags: 0x{:.04x}\n", self.xloadflags)?;
        write!(f, "  Max Cmdline Size: {} bytes\n", self.cmdline_size)?;
        write!(f, "  Payload Offset: 0x{:.08x}\n", self.payload_offset)?;
        write!(f, "  Payload length: {} bytes\n", self.payload_length)?;
        write!(f, "  init size: {} bytes\n", self.init_size)?;
        if let Some(ref address) = self.pref_address {
            write!(f, "  Preferred load address: 0x{:.016x}\n", address)?;
        }
        write!(f, "  EFI Handover Offset: 0x{:.08x}\n", self.handover_offset)?;
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
