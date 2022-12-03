
use core::hash::Hash;

use crate::{prelude::*, basic_consts::*};

use riscv::register::{self, satp::Mode};
use const_default::ConstDefault;
use bitflags::bitflags;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualAddress(pub u64);
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    pub const fn offset(self) -> u64 {
        self.0 & BITS_12
    }

    pub const fn ppn(self) -> u64 {
        (self.0 & BITS_55 & !BITS_12) >> 12
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rsw {
    Rsw0 = 0,
    Rsw1 = 1,
    Rsw2 = 2,
    Rsw3 = 3
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pbmt {
    Pma = 0,
    Nc = 1,
    Io = 2,
    _Reserved = 3
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {

    /// Create struct accounting checking the mode is supported.
    /// The spec marks `write && !read` as "Reserved".
    pub fn try_new(read: bool, write: bool, execute: bool) -> Option<Self> {        
        if write && !read {
            None 
        } else {
            Some(Self { read, write, execute })
        }
    }

    pub fn is_none(&self) -> bool {
        !(self.read || self.write || self.execute)
    }
}

pub fn print_current_page_table() {
    let satp = register::satp::read();
     
    println!("PageTable: {{");
    println!("  mode: {:?}", satp.mode());
    println!("  asid: {:?}", satp.asid());
    println!("  ppn:  {:?}", satp.ppn());
    println!("}}");
    if satp.mode() == Mode::Bare {
        println!("Base mapping no more details.");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [Entry; 512],
}

impl ConstDefault for PageTable {
    const DEFAULT: Self = Self {
        entries: [Entry::DEFAULT; 512]
    };
}

const MEGA_PAGE_SIZE: u64 =          0x200000;
const GIGA_PAGE_SIZE: u64 =        0x40000000;
const TERA_PAGE_SIZE: u64 =   0x2000000000000;
const PETA_PAGE_SIZE: u64 = 0x400000000000000;

pub fn dumb_map() -> PageTable {
    let mut pt = PageTable::DEFAULT;
    pt.entries[0] = Entry::builder()
        .for_offset(0)
        .valid(true)
        .readable(true)
        .writable(true)
        .executable(true)
        .build();
    pt.entries[1] = Entry::builder()
        .for_offset(0x40000000)
        .valid(true)
        .readable(true)
        .writable(true)
        .executable(true)
        .build();

    pt
}

bitflags! {
    struct VirtualAddressMask : u64 {
        const PAGE_OFFSET = BITS_12;
        const VPN_0 = BITS_9 << 12;
        const VPN_1 = BITS_9 << 21;
        const VPN_2 = BITS_9 << 30;
    }
}

bitflags! {
    struct PhysicalAddressMask : u64 {
        const PAGE_OFFSET = BITS_12;
        const PPN_0 = BITS_9 << 12;
        const PPN_1 = BITS_9 << 21;
        const PPN_2 = BITS_26 << 30;
    }
}

bitflags! {
    pub struct Entry : u64 {
        #[doc = "Entry is valid"]
        const V = 1 << 0;
        #[doc = "Is a leaf page and is readable"]
        const R = 1 << 1;
        #[doc = "Is a leaf page and is writable"]
        const W = 1 << 2;
        #[doc = "Is a leaf page and is executable"]
        const X = 1 << 3;
        #[doc = "User accessible. If `SUM` is set in sstatus. Then this page is NOT readable from S-mode."]
        const U = 1 << 4;
        #[doc = "Global page. Is present in all page tables. Affects TLB performance."]        
        const G = 1 << 5;
        #[doc = "Page has been read since A was last cleared. Page might have been speculartively read."]
        const A = 1 << 6;
        #[doc = "Page has been modified since D was last cleared. Must be strickly written to and not speculatively touched."]
        const D = 1 << 7;
        #[doc = "Bits free for use by superviser mode software. Hardare will no touch this."]        
        const RSW = BITS_2 << 8;
        #[doc = "Physical page number lowest 9 bits. Must be zero in mega or giga pages."]
        const PPN_0 = BITS_9 << 10;
        #[doc = "Physical page number second lowest 9 bits. Must be zero in giga pages."]
        const PPN_1 = BITS_9 << 19;
        #[doc = "Highest 26 bits in physical page number."]
        const PPN_2 = BITS_26 << 28;        

        #[doc = "Page caching mode. Specified by Svpbmt extension"]
        const PBMT  = BITS_2 << 61;
        #[doc = "Use for NAPOT entries. Specified by Svnapot"]
        const N     = 1 << 63;
    }
}

impl Entry {
    pub fn builder() -> EntryBuilder {
        EntryBuilder { entry: Entry::empty() }
    }
}

pub struct EntryBuilder {
    entry: Entry,
}

impl EntryBuilder {
    pub fn for_offset(mut self, offset: u64) -> Self {
        let pa = PhysicalAddress(offset);
        self.entry.remove(Entry::PPN_0);
        self.entry.remove(Entry::PPN_1);
        self.entry.remove(Entry::PPN_2);
        self.entry &= Entry::from_bits(pa.ppn_0() << 10).unwrap();
        self.entry &= Entry::from_bits(pa.ppn_1() << 19).unwrap();
        self.entry &= Entry::from_bits(pa.ppn_2() << 28).unwrap();
        self
    }

    pub fn valid(mut self, preset: bool) -> Self {
        self.entry.set(Entry::V, preset);
        self
    }
    pub fn readable(mut self, preset: bool) -> Self {
        self.entry.set(Entry::R, preset);
        self
    }
    pub fn writable(mut self, preset: bool) -> Self {
        self.entry.set(Entry::W, preset);
        self
    }
    pub fn executable(mut self, preset: bool) -> Self {
        self.entry.set(Entry::X, preset);
        self
    }
    pub fn build(self) -> Entry {
        self.entry
    }
}

impl ConstDefault for Entry {
    const DEFAULT: Self = Entry::empty();
}

impl VirtualAddress {
    fn page_offset(self) -> u64 {
        self.0 & VirtualAddressMask::PAGE_OFFSET.bits()
    }
    pub fn vpn_0(self) -> u64 {
        (self.0 & VirtualAddressMask::VPN_0.bits()) >> 12
    }
    pub fn vpn_1(self) -> u64 {
        (self.0 & VirtualAddressMask::VPN_1.bits()) >> 21
    }
    pub fn vpn_2(self) -> u64 {
        (self.0 & VirtualAddressMask::VPN_2.bits()) >> 30
    }
}

impl PhysicalAddress {
    pub fn page_offset(self) -> u64 {
        self.0 & PhysicalAddressMask::PAGE_OFFSET.bits()
    }
    pub fn ppn_0(self) -> u64 {
        (self.0 & PhysicalAddressMask::PPN_0.bits()) >> 12
    }
    pub fn ppn_1(self) -> u64 {
        (self.0 & PhysicalAddressMask::PPN_1.bits()) >> 21
    }
    pub fn ppn_2(self) -> u64 {
        (self.0 & PhysicalAddressMask::PPN_2.bits()) >> 30
    }
}

impl Entry {
    pub fn ppn_0(self) -> u64 {
        (self & Self::PPN_0).bits() >> 10
    }
    pub fn ppn_1(self) -> u64 {
        (self & Self::PPN_1).bits() >> 19
    }
    pub fn ppn_2(self) -> u64 {
        (self & Self::PPN_2).bits() >> 28
    }
}

impl Entry {
    pub fn rsw(self) -> Rsw {
        match (self & Self::RSW).bits() >> 8 {
            0 => Rsw::Rsw0,
            1 => Rsw::Rsw1,
            2 => Rsw::Rsw2,
            3 => Rsw::Rsw3,
            _ => unreachable!(),
        }
    }

    pub fn pbmt(self) -> Pbmt {
        match (self & Self::PBMT).bits() >> 61 {
            0 => Pbmt::Pma,
            1 => Pbmt::Nc,
            2 => Pbmt::Io,
            3 => Pbmt::_Reserved,
            _ => unreachable!(),
        }
    }

    pub fn valid(self) -> bool {
        (self & Self::V).bits() != 0
    }

    pub fn permissions(self) -> Permissions {
        let read = (self & Self::R).is_empty();
        let write = (self & Self::W).is_empty();
        let execute = (self & Self::X).is_empty();
        Permissions { read, write, execute }
    }

    pub fn user_accessible(self) -> bool {
        (self & Self::U).is_empty()
    }

    pub fn global(self) -> bool {
        (self & Self::G).is_empty()
    }

    pub fn accessed(self) -> bool {
        (self & Self::A).is_empty()
    }

    pub fn dirty(self) -> bool {
        (self & Self::D).is_empty()
    }
}

