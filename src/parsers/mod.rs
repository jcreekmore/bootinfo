use bytes;
use std::fmt::Display;

pub trait BootInfo: Display {}
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

pub mod multiboot1;
pub mod multiboot2;

pub fn register() -> Vec<Descriptor> {
    let mut descs = vec![];
    multiboot1::register(&mut descs);
    multiboot2::register(&mut descs);
    descs
}
