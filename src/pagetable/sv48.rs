use core::{
    alloc::Layout,
    borrow::Borrow,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    iter::from_fn,
    mem::forget,
    ops::Deref,
};

use crate::{basic_consts::*, prelude::*};

use bitflags::bitflags;
use const_default::ConstDefault;
use riscv::register::{
    self,
    satp::{self, Mode},
};
use smallvec::SmallVec;

use super::memory_map::{MemoryRegions, Permission};

pub const PAGE_SIZE: u64 = 0x1000;
pub const MEGA_PAGE_SIZE: u64 = 0x200000;
pub const GIGA_PAGE_SIZE: u64 = 0x40000000;
pub const TERA_PAGE_SIZE: u64 = 0x2000000000000;
pub const PETA_PAGE_SIZE: u64 = 0x400000000000000;

/// Mask to access or clear offsets within a page.
const OFFSET_MASK: u64 = BITS_12;
/// Mask to access bits used to access page number of an address.
const PAGE_NUMBER_MASK: u64 =
    EntryFlags::PPN_0.bits | EntryFlags::PPN_1.bits | EntryFlags::PPN_2.bits;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualAddress(pub u64);
impl VirtualAddress {
    const VPN_0_MASK: u64 = BITS_9 << 12;
    const VPN_1_MASK: u64 = BITS_9 << 21;
    const VPN_2_MASK: u64 = BITS_9 << 30;
    const VPN_3_MASK: u64 = BITS_9 << 39;
    const VPN_MASK: u64 = Self::VPN_0_MASK | Self::VPN_1_MASK | Self::VPN_2_MASK | Self::VPN_3_MASK;

    /// Lowest address. Zero.
    const MIN_ADDRESS: u64 = 0;
    /// Highest address. 2^48 - 1. This will change between paging systems.
    const MAX_ADDRESS: u64 = (1 << 48) - 1;

    pub const fn new(address: u64) -> Option<VirtualAddress> {
        if address & !(Self::VPN_MASK | OFFSET_MASK) != 0 {
            None
        } else {
            Some(VirtualAddress(address))
        }
    }

    /// Offset with a page. In range `0..4096`
    pub const fn offset_in_vpn(self) -> u64 {
        self.0 & OFFSET_MASK
    }

    /// Address of page containing address.
    pub const fn page_address(self) -> VirtualAddress {
        VirtualAddress(self.0 & Self::VPN_MASK)
    }

    /// Virtual page number.
    pub const fn vpn(self) -> u64 {
        (self.0 & Self::VPN_MASK) >> 12
    }

    /// Virtual page number of level 0.
    pub const fn vpn_0(self) -> u64 {
        (self.0 & Self::VPN_0_MASK) >> 12
    }

    /// Virtual page number of level 1.
    pub const fn vpn_1(self) -> u64 {
        (self.0 & Self::VPN_1_MASK) >> 21
    }

    /// Virtual page number of level 2.
    pub const fn vpn_2(self) -> u64 {
        (self.0 & Self::VPN_2_MASK) >> 30
    }

    /// Virtual page number of level 3.
    pub const fn vpn_3(self) -> u64 {
        (self.0 & Self::VPN_3_MASK) >> 39
    }

    pub const fn vpn_for_level(self, level: PageLevel) -> u64 {
        match level {
            PageLevel::Level0 => self.vpn_0(),
            PageLevel::Level1 => self.vpn_1(),
            PageLevel::Level2 => self.vpn_2(),
            PageLevel::Level3 => self.vpn_3(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    /// Offset within the current physical page (frame)
    pub const fn offset_in_ppn(self) -> u64 {
        self.0 & OFFSET_MASK
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

#[derive(Debug)]
pub struct PageTableRoot {
    root: Box<PageTable>,
}

impl PageTableRoot {
    pub fn get_mut(&mut self) -> PageTableMut<'_> {
        PageTableMut {
            table: &mut self.root,
            level: PageLevel::Level3,
        }
    }

    pub fn map_addr(&mut self, addr: PhysicalAddress, to: VirtualAddress, perm: Permission) {
        self.get_mut().map_addr(addr, to, perm)
    }

    pub(crate) fn new() -> Self {
        PageTableRoot {
            root: PageTable::allocate(),
        }
    }

    pub(crate) fn map_all(&mut self, memory_regions: MemoryRegions) {
        for (addr, perm) in memory_regions.iter_pages() {
            self.map_addr(PhysicalAddress(addr.0), addr, perm);
        }
    }

    pub fn print(&self) {
        println!("Root page:");
        self.root.print();
    }

    pub unsafe fn set_satp(&mut self, asid: u32) {
        let root_addr = (&*self.root) as *const PageTable as u64;
        // Update page table
        let pa = PhysicalAddress(root_addr);
        let ppn = pa.ppn();
        satp::set(satp::Mode::Sv48, asid as usize, ppn as usize);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [Entry; PAGE_ENTRIES],
}

impl PageTable {
    /// Allocate a new page table. All entries are zero. This ensures it's alligned correctly and isn't moved accidentally.
    pub fn allocate() -> Box<Self> {
        let new = Box::new(PageTable {
            entries: [Entry::DEFAULT; PAGE_ENTRIES],
        });
        println!("INFO: allocated {:08x}", new.address());
        new
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.flags().valid())
    }

    /// Free a page table. This will only succeed if the page table is all zero.
    pub fn try_free(mut self: Box<Self>) -> Result<(), Box<Self>> {
        if self.is_empty() {
            unsafe {
                let layout = Layout::new::<Self>();
                unsafe {
                    alloc::alloc::dealloc(&mut *self as *mut Self as *mut u8, layout);
                }
                forget(self);
                Ok(())
            }
        } else {
            Err(self)
        }
    }

    /// Get the address of the page table.
    pub fn address(&self) -> u64 {
        let addr = self as *const _;
        addr as usize as u64
    }

    pub fn print(&self) {
        for
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        // Because page table can have children which may have complex Drop logic, we don't free them for now.
        panic!("ERROR: leaked PageTable {:08x}", self.address());
    }
}

impl Debug for PageTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct("PageTable");

        // Length of "[512]" in utf-8 is 5 bytes.
        let mut name_buffer = String::with_capacity(5);

        let mut count = 0;
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let flags = entry.flags();
            if flags.valid() {
                count += 1;
                core::fmt::write(&mut name_buffer, format_args!("[{}]", i));
                debug.field(&name_buffer, &entry);
                name_buffer.clear();
            }
        }

        if count < self.entries.len() {
            debug.finish_non_exhaustive()
        } else {
            debug.finish()
        }
    }
}


#[derive(Debug)]
pub struct PageTableRef<'a> {
    table: &'a PageTable,
    level: PageLevel,
}

#[derive(Debug)]
pub struct PageTableRefEntry<'a> {
    table: &'a PageTable,
    level: PageLevel,
    index: usize,
}

impl<'a> PageTableRef<'a> {
    pub fn new(table: &'a PageTable, level: PageLevel) -> Self {
        Self { table, level }
    }

    pub fn level(&self) -> PageLevel {
        self.level
    }

    pub fn entry(&self, index: impl Into<usize>) -> PageTableRefEntry {
        assert!(index.into() < PAGE_ENTRIES);
        PageTableRefEntry {
            table: self.table,
            level: self.level,
            index: index.into()
        }
    }

    pub fn child_table(&self, index: usize) -> Option<PageTableRef<'a>> {
        todo!()
    }
}

impl<'a> PageTableRefEntry<'a> {
    pub fn flags(&self) -> EntryFlags {
        self.table.entries[self.index].flags()
    }

    pub fn valid(&self) -> bool {
        self.table.entries[self.index].flags().valid()
    }

    pub fn child(&self) -> Option<PageTableRef<'a>> {
        match (self.valid(), self.level.down()) {
            (true, Some(level)) => {
                let address = self.table.entries[self.index].address();
                let table = unsafe { &*(address.0 as *const PageTable) };
                Some(PageTableRef { table, level })
            }
            _ => None
        }
    }
}

#[derive(Debug)]
pub struct PageTableMut<'a> {
    /// Reference to page table.
    table: &'a mut PageTable,
    /// Level in page table. 0 is the leaf and refer to 4k pages. 3 is the highest in sv48.
    level: PageLevel,
}

#[derive(Debug)]
pub struct PageTableMutEntry<'a> {
    table: &'a mut PageTable,
    level: PageLevel,
    index: usize,
}

impl<'a> PageTableMut<'a> {
    pub fn new(table: &'a mut PageTable, level: PageLevel) -> Self {
        Self { table, level }
    }

    pub fn level(&self) -> PageLevel {
        self.level
    }

    pub fn entry_mut(&mut self, index: impl Into<usize>) -> PageTableMutEntry {
        PageTableMutEntry {
            table: self.table,
            index: index.into(),
            level: self.level,
        }
    }

    pub fn child_table(&mut self, index: usize) -> Option<PageTableMut<'a>> {
        let entry = self.table.entries[index];
        match (self.level.down(), entry.flags().valid()) {
            (Some(level), true) => {
                let addr = entry.address();
                let table = unsafe { &mut *(addr.0 as *mut PageTable) };
                Some(PageTableMut::new(table, level))
            }
            _ => None,
        }
    }

    pub fn map_addr(&mut self, addr: PhysicalAddress, to: VirtualAddress, perm: Permission) {
        //println!("map_addr: self={:?}, addr={:?}, to={:?}, perm={:?}",self, addr, to, perm);
        if self.level().bottom() {
            let mut entry = self.entry_mut(to.vpn_for_level(self.level()));
            if entry.valid() {
                panic!("ERROR: page already mapped");
            }
            entry.set(addr, perm);
        } else {
            let mut entry = self.entry_mut(to.vpn_for_level(self.level()));
            if !entry.valid() {
                let flags = EntryFlags::builder().with_perms(perm).valid(true).build();
                entry.insert(PageTable::allocate(), flags);
            }

            match entry.child() {
            Some(mut child) => child.map_addr(addr, to, perm),
            None => panic!("Expected child after inserting child page. entry={:?}, addr={:?}, to={:?}, perm={:?}", entry, addr, to, perm),
        }
        }
    }
}

impl<'a> PageTableMutEntry<'a> {
    pub fn flags(&self) -> EntryFlags {
        self.table.entries[self.index].flags()
    }

    pub fn valid(&self) -> bool {
        self.table.entries[self.index].flags().valid()
    }

    pub fn insert(&mut self, page: Box<PageTable>, flags: EntryFlags) {
        let addr = PhysicalAddress(page.address());
        self.table.entries[self.index] = Entry::new(addr, flags);
        forget(page);
    }

    pub fn child(&mut self) -> Option<PageTableMut<'a>> {
        match (self.valid(), self.level.down()) {
            (true, Some(level)) => {
                let address = self.table.entries[self.index].address();
                let table = unsafe { &mut *(address.0 as *mut PageTable) };
                Some(PageTableMut { table, level })
            }
            _ => None,
        }
    }

    fn set(&mut self, addr: PhysicalAddress, perm: Permission) {
        if self.valid() {
            panic!("ERROR: page already mapped");
        }
        let flags = EntryFlags::builder()
            .readable(perm.readable())
            .writable(perm.writable())
            .executable(perm.executable())
            .build();
        self.table.entries[self.index] = Entry::new(addr, flags);
    }
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
                let flags = entry.flags();
                if !flags.valid() {
                    continue;
                }

                if flags.is_leaf() {
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

pub fn map(table: &mut PageTableRoot, map: &MemoryRegions) {
    println!("Mapping memory");
    for (addr, perm) in map.iter_pages() {}
}

/// Maps first 4 GiB using big pages. All are R|W|X
pub fn place_dumb_map(map: &mut PageTable) {
    map.entries = [Entry::empty(); 512];
    for i in 0..4 {
        let flags = EntryFlags::builder()
            .valid(true)
            .readable(true)
            .writable(true)
            .executable(true)
            .build();

        map.entries[i] = Entry::new(PhysicalAddress(i as u64 * 0x40000000), flags);
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

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Entry(pub u64);

impl ConstDefault for Entry {
    const DEFAULT: Self = Self::empty();
}

impl Entry {
    /// Construct the empty entry. Default state for page table entries.
    /// All bits are zero.
    pub const fn empty() -> Self {
        Entry(0)
    }

    /// Contstruct entry from physical page number and flags.
    pub const fn new(address: PhysicalAddress, flags: EntryFlags) -> Entry {
        let bits = address.0 | flags.bits;
        Entry(bits)
    }

    /// Get the physical page number address refers to.
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & EntryFlags::PPN.bits)
    }

    /// Gives the flags set in the entry. ie. All bits except the address.
    pub fn flags(&self) -> EntryFlags {
        EntryFlags {
            bits: self.0 & EntryFlags::FLAGS.bits,
        }
    }

    /// Returns true if the entry is all zeros.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let address = self.address();
        let flags = self.flags();
        f.debug_tuple("Entry")
            .field(&format_args!("0x{:08x}", address.0))
            .field(&flags)
            .finish()
    }
}

bitflags! {
    pub struct EntryFlags : u64 {
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

        #[doc = "Mask to access only flags without address"]
        const FLAGS = Self::V.bits | Self::R.bits | Self::W.bits | Self::X.bits | Self::U.bits | Self::G.bits | Self::A.bits | Self::A.bits | Self::D.bits | Self::PBMT.bits | Self::N.bits;

        #[doc = "Mask to access entire PPN"]
        const PPN = Self::PPN_0.bits | Self::PPN_1.bits | Self::PPN_2.bits;
    }
}

impl EntryFlags {
    pub fn builder() -> EntryBuilder {
        EntryBuilder {
            entry: EntryFlags::empty(),
        }
    }

    pub fn just_flags(&self) -> EntryFlags {
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

impl Default for EntryFlags {
    fn default() -> Self {
        Self {
            bits: Default::default(),
        }
    }
}

impl ConstDefault for EntryFlags {
    const DEFAULT: Self = EntryFlags::empty();
}

pub struct EntryBuilder {
    entry: EntryFlags,
}

impl EntryBuilder {
    pub fn for_offset(mut self, offset: u64) -> Self {
        let pa = PhysicalAddress(offset);
        self.entry.remove(EntryFlags::PPN_0);
        self.entry.remove(EntryFlags::PPN_1);
        self.entry.remove(EntryFlags::PPN_2);
        self.entry &= EntryFlags::from_bits(pa.ppn_0() << 10).unwrap();
        self.entry &= EntryFlags::from_bits(pa.ppn_1() << 19).unwrap();
        self.entry &= EntryFlags::from_bits(pa.ppn_2() << 28).unwrap();
        self
    }

    pub fn valid(mut self, preset: bool) -> Self {
        self.entry.set(EntryFlags::V, preset);
        self
    }
    pub fn readable(mut self, preset: bool) -> Self {
        self.entry.set(EntryFlags::R, preset);
        self
    }
    pub fn writable(mut self, preset: bool) -> Self {
        self.entry.set(EntryFlags::W, preset);
        self
    }
    pub fn executable(mut self, preset: bool) -> Self {
        self.entry.set(EntryFlags::X, preset);
        self
    }

    fn with_perms(mut self, perm: Permission) -> Self {
        self.readable(perm.readable())
            .writable(perm.writable())
            .executable(perm.executable())
    }

    pub fn build(self) -> EntryFlags {
        self.entry
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PageLevel {
    Level0,
    Level1,
    Level2,
    Level3,
}

impl PageLevel {
    pub const fn up(self) -> Option<PageLevel> {
        match self {
            PageLevel::Level0 => Some(PageLevel::Level1),
            PageLevel::Level1 => Some(PageLevel::Level2),
            PageLevel::Level2 => Some(PageLevel::Level3),
            PageLevel::Level3 => None,
        }
    }

    pub const fn down(self) -> Option<PageLevel> {
        match self {
            PageLevel::Level0 => None,
            PageLevel::Level1 => Some(PageLevel::Level0),
            PageLevel::Level2 => Some(PageLevel::Level1),
            PageLevel::Level3 => Some(PageLevel::Level2),
        }
    }

    pub fn top(self) -> bool {
        self == PageLevel::Level2
    }

    pub fn bottom(self) -> bool {
        self == PageLevel::Level0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BigPage {
    Page(u64),
    MegaPage(u64),
    GigaPage(u64),
    TeraPage(u64),
    // PetaPage(u64),
}

impl Display for BigPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            BigPage::Page(pos) => write!(f, "Page@{:x}", pos),
            BigPage::MegaPage(pos) => write!(f, "MegaPage@{:x}", pos),
            BigPage::GigaPage(pos) => write!(f, "GigaPage@{:x}", pos),
            BigPage::TeraPage(pos) => write!(f, "TeraPage@{:x}", pos),
        }
    }
}

pub const PAGE_LEVELS: [(PageLevel, u64); 4] = [
    (BigPage::Page(0).level(), BigPage::Page(0).size()),
    (BigPage::MegaPage(0).level(), BigPage::MegaPage(0).size()),
    (BigPage::GigaPage(0).level(), BigPage::GigaPage(0).size()),
    (BigPage::TeraPage(0).level(), BigPage::TeraPage(0).size()),
];

impl BigPage {
    pub const fn new(level: PageLevel, address: u64) -> BigPage {
        match level {
            PageLevel::Level0 => BigPage::Page(address),
            PageLevel::Level1 => BigPage::MegaPage(address),
            PageLevel::Level2 => BigPage::GigaPage(address),
            PageLevel::Level3 => BigPage::TeraPage(address),
        }
    }

    pub const fn level(self) -> PageLevel {
        match self {
            BigPage::Page(_) => PageLevel::Level0,
            BigPage::MegaPage(_) => PageLevel::Level1,
            BigPage::GigaPage(_) => PageLevel::Level2,
            BigPage::TeraPage(_) => PageLevel::Level3,
        }
    }

    pub const fn size(self) -> u64 {
        match self {
            BigPage::Page(_) => PAGE_SIZE,
            BigPage::MegaPage(_) => MEGA_PAGE_SIZE,
            BigPage::GigaPage(_) => GIGA_PAGE_SIZE,
            BigPage::TeraPage(_) => TERA_PAGE_SIZE,
        }
    }

    pub const fn position(self) -> u64 {
        match self {
            BigPage::Page(n)
            | BigPage::MegaPage(n)
            | BigPage::GigaPage(n)
            | BigPage::TeraPage(n) => n,
        }
    }

    pub fn page_for(position: u64, at_most: u64) -> BigPage {
        for (level, size) in PAGE_LEVELS.iter().rev() {
            if at_most >= *size && (position & (size - 1) == 0) {
                match level {
                    PageLevel::Level0 => return BigPage::Page(position),
                    PageLevel::Level1 => return BigPage::MegaPage(position),
                    PageLevel::Level2 => return BigPage::GigaPage(position),
                    PageLevel::Level3 => return BigPage::TeraPage(position),
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
