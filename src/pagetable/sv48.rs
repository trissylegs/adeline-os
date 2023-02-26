use core::{hash::Hash, iter::from_fn, fmt::{Display, Formatter}};

use crate::{
    basic_consts::*,
    prelude::*,
};

use bitflags::bitflags;
use const_default::ConstDefault;
use riscv::register::{self, satp::Mode};
use smallvec::SmallVec;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualAddress(pub u64);
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
    pub fn vpn_3(self) -> u64 {
        (self.0 & VirtualAddressMask::VPN_3.bits()) >> 39
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    /// Offset within the current physical page (frame)
    pub const fn offset_in_ppn(self) -> u64 {
        self.0 & BITS_12
    }

    /// Physical page (or frame) number.
    pub const fn ppn(self) -> u64 {
        (self.0 & BITS_55 & !BITS_12) >> 12
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
    pub fn ppn_3(self) -> u64 {
        (self.0 & PhysicalAddressMask::PPN_3.bits()) >> 39
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rsw {
    Rsw0 = 0,
    Rsw1 = 1,
    Rsw2 = 2,
    Rsw3 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pbmt {
    Pma = 0,
    Nc = 1,
    Io = 2,
    _Reserved = 3,
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
            Some(Self {
                read,
                write,
                execute,
            })
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

pub const PAGE_ENTRIES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [Entry; PAGE_ENTRIES],
}

pub struct PageTableRef<'a> {
    /// Reference to page table.
    table: &'a PageTable,
    /// Level in page table. 0 is the leaf and refer to 4k pages. 3 is the highest in sv48.
    level: PageLevel,
    /// Virtual address offset.
    offset: VirtualAddress,
}

impl<'a> PageTableRef<'a> {
    pub fn get_entry(&self, index: usize) -> EntryKind<'a> {
        let entry = self.table.entries[index];
        if !entry.valid() {
            EntryKind::Empty
        } else if entry.permissions().is_none() {
            todo!()
        } else {
            let addr = entry.address();
            let page = BigPage::new(self.level, addr.0);
            EntryKind::Present(page)
        }
    }
}

pub enum EntryKind<'a> {
    Empty,
    Present(BigPage),
    ChildTable(u64),
    _ChildTable(PageTableRef<'a>)
}

pub fn iter_pages<'a>(pt: &'a PageTable) -> impl Iterator<Item = BigPage> + 'a {
    let mut todo: SmallVec<[&PageTable; 3]> = SmallVec::new();
    let mut level = PageLevel::Level2;
    let mut index = 0;
    todo.push(pt);
    from_fn(move || {
        while !todo.is_empty() {
            let current = *todo.last().unwrap();
            while index < 512 {
                let curr_index = index;
                index += 1;
                let entry = current.entries[curr_index];
                if !entry.valid() {
                    continue;
                }

                if entry.is_leaf() {
                    let addr = entry.address();
                    return Some(BigPage::new(level, addr.0));
                }

                let addr = entry.address();
                // UNSAFELY Assuming Identity map.
                let ptr = addr.0 as *const PageTable;
                unsafe {
                    let next = &*ptr;
                    level = level.down().unwrap();
                    todo.push(next);
                }
            }
            todo.pop();
            level = level.up().unwrap();
        }
        return None;
    })
}

impl ConstDefault for PageTable {
    const DEFAULT: Self = Self {
        entries: [Entry::DEFAULT; 512],
    };
}

pub const PAGE_SIZE: u64 = 0x1000;
pub const MEGA_PAGE_SIZE: u64 = 0x200000;
pub const GIGA_PAGE_SIZE: u64 = 0x40000000;
pub const TERA_PAGE_SIZE: u64 = 0x2000000000000;
pub const PETA_PAGE_SIZE: u64 = 0x400000000000000;

pub fn place_dump_map(map: &mut PageTable) {
    *map = PageTable::DEFAULT;
    for i in 0..4 {
        map.entries[i] = Entry::builder()
            .for_offset((i * 0x40000000) as u64)
            .valid(true)
            .readable(true)
            .writable(true)
            .executable(true)
            .build();
    }
}

bitflags! {
    struct VirtualAddressMask : u64 {
        const PAGE_OFFSET = BITS_12;
        const VPN_0 = BITS_9 << 12;
        const VPN_1 = BITS_9 << 21;
        const VPN_2 = BITS_9 << 30;
        const VPN_3 = BITS_9 << 39;
    }
}

bitflags! {
    struct PhysicalAddressMask : u64 {
        const PAGE_OFFSET = BITS_12;
        const PPN_0 = BITS_9 << 12;
        const PPN_1 = BITS_9 << 21;
        const PPN_2 = BITS_9 << 30;
        const PPN_3 = BITS_17 << 39;
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
        #[doc = "Page has been read since A was last cleared. Page might have been speculatively read."]
        const A = 1 << 6;
        #[doc = "Page has been modified since D was last cleared. Must be actually written to and not just speculatively touched."]
        const D = 1 << 7;
        #[doc = "Bits free for use by supervisor mode software. Hardware will no touch this."]
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

        const FLAGS = Self::V.bits | Self::R.bits | Self::W.bits | Self::X.bits | Self::U.bits | Self::G.bits | Self::A.bits | Self::A.bits | Self::D.bits;
        const PPN = Self::PPN_0.bits | Self::PPN_1.bits | Self::PPN_2.bits;
    }
}

impl Entry {
    pub fn builder() -> EntryBuilder {
        EntryBuilder {
            entry: Entry::empty(),
        }
    }

    pub fn just_flags(&self) -> Entry {
        *self & Self::FLAGS
    }
    pub fn ppn_0(self) -> u64 {
        (self & Self::PPN_0).bits() >> 10
    }
    pub fn ppn_1(self) -> u64 {
        (self & Self::PPN_1).bits() >> 19
    }
    pub fn ppn_2(self) -> u64 {
        (self & Self::PPN_2).bits() >> 28
    }

    pub fn address(self) -> PhysicalAddress {
        PhysicalAddress((self & Self::PPN).bits())
    }

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

    pub fn is_leaf(self) -> bool {
        self.permissions().is_none()
    }

    pub fn permissions(self) -> Permissions {
        let read = (self & Self::R).is_empty();
        let write = (self & Self::W).is_empty();
        let execute = (self & Self::X).is_empty();
        Permissions {
            read,
            write,
            execute,
        }
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

impl Default for Entry {
    fn default() -> Self {
        Self {
            bits: Default::default(),
        }
    }
}

impl ConstDefault for Entry {
    const DEFAULT: Self = Entry::empty();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PageLevel {
    Level0,
    Level1,
    Level2,
}

impl PageLevel {
    pub const fn up(self) -> Option<PageLevel> {
        match self {
            PageLevel::Level0 => Some(PageLevel::Level1),
            PageLevel::Level1 => Some(PageLevel::Level2),
            PageLevel::Level2 => None,
        }
    }

    pub const fn down(self) -> Option<PageLevel> {
        match self {
            PageLevel::Level0 => None,
            PageLevel::Level1 => Some(PageLevel::Level0),
            PageLevel::Level2 => Some(PageLevel::Level1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BigPage {
    Page(u64),
    MegaPage(u64),
    GigaPage(u64),
    // TeraPage(u64),
    // PetaPage(u64),
}

impl Display for BigPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            BigPage::Page(pos) => write!(f, "Page@{:x}", pos),
            BigPage::MegaPage(pos) => write!(f, "MegaPage@{:x}", pos),
            BigPage::GigaPage(pos) => write!(f, "GigaPage@{:x}", pos),
        }
    }
}

pub const PAGE_LEVELS: [(PageLevel, u64); 2] = [
    (BigPage::Page(0).level(), BigPage::Page(0).size()),
    (BigPage::MegaPage(0).level(), BigPage::MegaPage(0).size()),
];


impl BigPage {
    pub const fn new(level: PageLevel, address: u64) -> BigPage {
        match level {
            PageLevel::Level0 => BigPage::Page(address),
            PageLevel::Level1 => BigPage::MegaPage(address),
            PageLevel::Level2 => BigPage::GigaPage(address),
        }
    }

    pub const fn level(self) -> PageLevel {
        match self {
            BigPage::Page(_) => PageLevel::Level0,
            BigPage::MegaPage(_) => PageLevel::Level1,
            BigPage::GigaPage(_) => PageLevel::Level2,
        }
    }

    pub const fn size(self) -> u64 {
        match self {
            BigPage::Page(_) => PAGE_SIZE,
            BigPage::MegaPage(_) => MEGA_PAGE_SIZE,
            BigPage::GigaPage(_) => GIGA_PAGE_SIZE,
        }
    }

    pub const fn position(self) -> u64 {
        match self {
            BigPage::Page(n)
            | BigPage::MegaPage(n)
            | BigPage::GigaPage(n)
            => n,
        }
    }


    pub fn page_for(position: u64, at_most: u64) -> BigPage {
        for (level, size) in PAGE_LEVELS.iter().rev() {
            if at_most >= *size && (position & (size - 1) == 0) {
                match level {
                    PageLevel::Level0 => return BigPage::Page(position),
                    PageLevel::Level1 => return BigPage::MegaPage(position),
                    PageLevel::Level2 => return BigPage::GigaPage(position),
                    // PageLevel::Level3 => return BigPage::TeraPage(position),
                    // PageLevel::Level4 => return BigPage::PetaPage(position),
                }
            }
        }
        panic!(
            "Invalid page spec: position: {:x}, at_most: {:x}",
            position, at_most
        );
    }
}