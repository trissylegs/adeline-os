use core::{any::type_name, fmt::{Debug, Display, Formatter}, hash::Hash, marker::PhantomData, mem, mem::ManuallyDrop};

use crate::{basic_consts::*, prelude::*, util::IndentPrint};

use bitflags::bitflags;
use const_default::ConstDefault;
use riscv::register::{
    self,
    satp::Mode,
};
use crate::pagetable::BigPage::GigaPage;

use super::memory_map::{MemoryRegions, Permission};

pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}
pub enum Level0 {}

pub trait TableLevel {
    const LEVEL: u8;

    fn level_vpn(v: &VirtualAddress) -> usize;
}

impl TableLevel for Level3 {
    const LEVEL: u8 = 3;
    fn level_vpn(v: &VirtualAddress) -> usize {
        v.vpn_3() as usize
    }
}
impl TableLevel for Level2 {
    const LEVEL: u8 = 2;

    fn level_vpn(v: &VirtualAddress) -> usize {
        v.vpn_2() as usize
    }
}
impl TableLevel for Level1 {
    const LEVEL: u8 = 1;

    fn level_vpn(v: &VirtualAddress) -> usize {
        v.vpn_1() as usize
    }
}
impl TableLevel for Level0 {
    const LEVEL: u8 = 0;

    fn level_vpn(v: &VirtualAddress) -> usize {
        v.vpn_0() as usize
    }
}

trait MapAddr: TableLevel + Sized {
    fn map_addr(
        table: &mut PageTable<Self>,
        addr: PhysicalAddress,
        to: VirtualAddress,
        perm: Permission,
    );

    fn print(table: &PageTable<Self>, virt: u64);
}

impl MapAddr for Level0 {
    fn map_addr(
        table: &mut PageTable<Level0>,
        addr: PhysicalAddress,
        to: VirtualAddress,
        perm: Permission,
    ) {
        let entry_index = Level0::level_vpn(&to);
        let flags = EntryFlags::builder()
            .valid(true)
            .with_permissions(perm)
            .build();
        let entry = Entry::new(addr, flags);
        // println!("Adding mapping {:?} for {:08x}", entry, to.0);
        table.entries[entry_index] = entry;
    }

    fn print(table: &PageTable<Self>, virt: u64) {
        return;
        // This is written independent so we can see where I fucked everything up.
        let mut writer = IndentPrint::new(2 * 3);
        let level = Level0::LEVEL;
        for (i, e) in table.entries.iter().enumerate() {
            let vpn = virt | ((i as u64) << (level * 9 + 12));

            let bits = e.bits();
            if bits != 0 {
                let v = bits & 1;
                let r = (bits >> 1) & 1;
                let w = (bits >> 2) & 1;
                let x = (bits >> 3) & 1;
                let u = (bits >> 4) & 1;
                let g = (bits >> 5) & 1;
                let a = (bits >> 6) & 1;
                let d = (bits >> 7) & 1;
                let rsw = (bits >> 8) & BITS_2;
                let ppn = bits & (BITS_44 << 10);
                let reserved = (bits >> 54) & BITS_7;
                let pbmt = (bits >> 61) & BITS_2;
                let n = (bits >> 63) & 1;

                writeln!(writer, "vpn=0x{vpn:08x} ppn=0x{ppn:08x} v={v} r={r} w={w} x={x} u={u} g={g} a={a} d={d} rsw={rsw} ppn=0x{ppn:08x} reserved=0x{reserved:x} pbmt={pbmt} n={n}")
                    .expect("writeln");
            }
        }
    }
}

impl<H: HierarchicalLevel> MapAddr for H
where
    H::Next: MapAddr,
{
    fn map_addr(
        table: &mut PageTable<H>,
        addr: PhysicalAddress,
        to: VirtualAddress,
        perm: Permission,
    ) {
        //println!("map_addr: self={:?}, addr={:?}, to={:?}, perm={:?}",self, addr, to, perm);
        let mut entry = table.entry_for_mut(to);

        match entry.child() {
            Some(child) => H::Next::map_addr(child, addr, to, perm),
            None => {
                let child = entry.insert_child_table(PageTable::allocate());
                H::Next::map_addr(child, addr, to, perm)
            }
        }
    }

    fn print(table: &PageTable<Self>, virt: u64) {
        // This written to be independent of the other page code so I can tell if screwed it up.
        let mut writer = IndentPrint::new(2 * (3 - H::LEVEL));
        let level = H::LEVEL as u64;

        let addr = (table as *const PageTable<Self>) as usize;
        for (i, e) in table.entries.iter().enumerate() {
            let vpn = virt | ((i as u64) << (level * 9));

            let bits = e.bits();
            if bits != 0 {
                let info = PageEntryInfo {
                    v: bits & 1,
                    r: (bits >> 1) & 1,
                    w: (bits >> 2) & 1,
                    x: (bits >> 3) & 1,
                    u: (bits >> 4) & 1,
                    g: (bits >> 5) & 1,
                    a: (bits >> 6) & 1,
                    d: (bits >> 7) & 1,
                    rsw: (bits >> 8) & 0b11,
                    ppn_0: (bits >> 10) & 0b111111111,
                    ppn_1: (bits >> 19) & 0b111111111,
                    ppn_2: (bits >> 28) & 0b111111111,
                    ppn_3: (bits >> 37) & 0b11111111111111111,
                    reserved: (bits >> 54) & 0b1111111,
                    pbmt: (bits >> 61) & 0b11,
                    n: (bits >> 63) & 1,
                };

                writeln!(writer, "0x{addr:08x} {info:?}").expect("writeln");

                // writeln!(writer, "addr=0x{addr:08x} vpn=0x{vpn:08x} ppn=0x{ppn:08x} v={v} r={r} w={w} x={x} u={u} g={g} a={a} d={d} rsw={rsw} ppn=0x{ppn:08x} reserved=0x{reserved:x} pbmt={pbmt} n={n}")
                //    .expect("writeln");

                if info.v != 0 && info.r == 0 && info.w == 0 && info.x == 0 {
                    let addr = e.address().0 as usize as *const PageTable<H::Next>;
                    let ptr = unsafe { &*addr };
                    H::Next::print(ptr, vpn);
                }
            }
        }
    }
}

#[derive(Debug)]
struct PageEntryInfo {
    v: u64,
    r: u64,
    w: u64,
    x: u64,
    u: u64,
    g: u64,
    a: u64,
    d: u64,
    rsw: u64,
    ppn_0: u64,
    ppn_1: u64,
    ppn_2: u64,
    ppn_3: u64,
    reserved: u64,
    pbmt: u64,
    n: u64,
}

pub trait HierarchicalLevel: TableLevel {
    type Next: TableLevel;
}

impl HierarchicalLevel for Level3 {
    type Next = Level2;
}
impl HierarchicalLevel for Level2 {
    type Next = Level1;
}
impl HierarchicalLevel for Level1 {
    type Next = Level0;
}

// 4,096 B4; 4 K
pub const PAGE_SIZE: u64 = 0x1000;
// 2,097,152 B; 2048 K, 2 M
pub const MEGA_PAGE_SIZE: u64 = 0x200000;
// 1,073,741,824 B; 1,048,576 K; 1024 M; 1 G
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
    root: Box<PageTable<Level3>>,
}

impl PageTableRoot {
    pub fn get_mut(&mut self) -> &mut PageTable<Level3> {
        &mut self.root
    }

    pub fn map_addr(&mut self, addr: PhysicalAddress, to: VirtualAddress, perm: Permission) {
        Level3::map_addr(&mut self.root, addr, to, perm)
    }

    pub(crate) fn new() -> Self {
        PageTableRoot {
            root: PageTable::allocate(),
        }
    }

    pub(crate) fn map_all(&mut self, memory_regions: MemoryRegions) {
        for region in memory_regions.iter_regions() {
            println!("Region: {:?}", region);
            for (addr, perm) in region.iter_pages() {
                self.map_addr(PhysicalAddress(addr.0), addr, perm);
            }
        }
    }

    pub fn dumb_map(&mut self) {
        println!("Mapping 4 giga pages.");
        let root = &mut *self.root;
        let flags = EntryFlags::V | EntryFlags::R | EntryFlags::W  | EntryFlags::X;

        let mut page = PageTable::<Level2>::allocate();

        for i in 0..512 {
            // let page = Box::new(PageTable::<Level2>)
            page.entries[i] = Entry::new(PhysicalAddress((i as u64) * GIGA_PAGE_SIZE), flags);
        }

        root.entries[0] = Entry::new(PhysicalAddress(page.address()), EntryFlags::V);
        mem::forget(page);
    }

    pub unsafe fn set_satp(&mut self, asid: u16) {
        let root_addr = (&*self.root) as *const PageTable<Level3> as u64;
        // Update page table
        let pa = PhysicalAddress(root_addr);
        let ppn = pa.ppn();
        const SV48: u64 = 9;
        let sapt_value = ppn | (asid as u64) << 44 | SV48 << 60;
        // set sapt register
        unsafe {
            core::arch::asm!("csrrw x0, satp, {0}", in(reg) sapt_value);
        }
    }

    pub fn print(&self) {
        println!("Page table root 0x{:08x}", self.root.address());
        Level3::print(&self.root, 0);
    }
}

#[derive(PartialEq, Eq, Hash)]
#[repr(C, align(4096))]
pub struct PageTable<L: TableLevel> {
    entries: [Entry; PAGE_ENTRIES],
    _p: PhantomData<L>,
}

impl<L: TableLevel> PageTable<L> {

    /// Allocate a new page table. All entries are zero. This ensures it's aligned correctly and isn't moved accidentally.
    pub fn allocate() -> Box<Self> {
        let new = Box::new(PageTable {
            entries: [Entry::DEFAULT; PAGE_ENTRIES],
            _p: PhantomData,
        });
        // println!("INFO: allocated {:08x}", new.address());
        new
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.flags().valid())
    }

    /// Free a page table. This will only succeed if the page table is all zero. Otherwise it will return the page back to you.
    pub fn try_free(self: Box<Self>) -> Result<(), Box<Self>> {
        if self.is_empty() {
            unsafe {
                let raw = Box::into_raw(self);
                let manual_drop = raw as *mut ManuallyDrop<PageTable<L>>;
                let _ = Box::from_raw(manual_drop);
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

    pub fn entry<'a>(&'a self, index: impl Into<usize>) -> PageTableRefEntry<'a, L> {
        let index = index.into();
        assert!(index < PAGE_ENTRIES);
        PageTableRefEntry {
            table: self,
            index: index,
        }
    }

    pub fn entry_mut<'a>(&'a mut self, index: impl Into<usize>) -> PageTableMutEntry<'a, L> {
        let index = index.into();
        assert!(index < PAGE_ENTRIES);
        PageTableMutEntry {
            table: self,
            index: index,
        }
    }

    fn entry_for_mut(&mut self, to: VirtualAddress) -> PageTableMutEntry<L> {
        self.entry_mut(L::level_vpn(&to))
    }
}

impl<L: TableLevel> Drop for PageTable<L> {
    fn drop(&mut self) {
        // Because page table can have children which may have complex Drop logic, we don't free them for now.
        panic!("ERROR: leaked PageTable {:08x}", self.address());
    }
}

impl<L: TableLevel> Debug for PageTable<L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct(type_name::<Self>());

        // Length of "[512]" in utf-8 is 5 bytes.
        let mut name_buffer = String::with_capacity(5);

        let mut count = 0;
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let flags = entry.flags();
            if flags.valid() {
                count += 1;
                core::fmt::write(&mut name_buffer, format_args!("[{}]", i))?;
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
pub struct PageTableRefEntry<'a, L: TableLevel> {
    table: &'a PageTable<L>,
    index: usize,
}

impl<'a, L: TableLevel> PageTableRefEntry<'a, L> {
    pub fn flags(&self) -> EntryFlags {
        self.table.entries[self.index].flags()
    }

    pub fn valid(&self) -> bool {
        self.table.entries[self.index].flags().valid()
    }
}

impl<'a, L: HierarchicalLevel> PageTableRefEntry<'a, L> {
    pub fn child(&self) -> Option<&'a PageTable<L::Next>> {
        if self.valid() {
            let address = self.table.entries[self.index].address();
            let table = unsafe { &*(address.0 as *const PageTable<_>) };
            Some(table)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PageTableMutEntry<'a, L: TableLevel> {
    table: &'a mut PageTable<L>,
    index: usize,
}

impl<'a, L: HierarchicalLevel> PageTableMutEntry<'a, L> {
    pub fn child(&mut self) -> Option<&mut PageTable<L::Next>> {
        let entry = self.table.entries[self.index];
        if entry.flags().valid() {
            let addr = entry.address();
            let table = unsafe { &mut *(addr.0 as *mut PageTable<_>) };
            Some(table)
        } else {
            None
        }
    }

    pub fn insert_child_table(
        &'a mut self,
        page: Box<PageTable<L::Next>>,
    ) -> &'a mut PageTable<L::Next> {
        let pointer = Box::into_raw(page);
        let addr = PhysicalAddress(pointer as usize as u64);
        let flags = EntryFlags::builder()
            .valid(true)
            .readable(false)
            .writable(false)
            .executable(false)
            .build();

        self.table.entries[self.index] = Entry::new(addr, flags);
        unsafe { &mut *pointer }
    }
}

impl<'a, L: TableLevel> PageTableMutEntry<'a, L> {
    pub fn flags(&self) -> EntryFlags {
        self.table.entries[self.index].flags()
    }

    pub fn valid(&self) -> bool {
        self.table.entries[self.index].flags().valid()
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

/// Maps first 4 GiB using big pages. All are R|W|X
///
/// # Warning
/// If root already has mapping's they will just be leaked here.
pub fn place_dumb_map(map: &mut PageTableRoot) {
    map.root.entries = [Entry::empty(); 512];
    for i in 0..4 {
        let flags = EntryFlags::builder()
            .valid(true)
            .readable(true)
            .writable(true)
            .executable(true)
            .build();

        map.root.entries[i] = Entry::new(PhysicalAddress(i as u64 * 0x40000000), flags);
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

    fn bits(&self) -> u64 {
        self.0
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
    pub fn builder() -> EntryFlagsBuilder {
        EntryFlagsBuilder {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryFlagsBuilder {
    entry: EntryFlags,
}

impl EntryFlagsBuilder {
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

    fn with_permissions(self, perm: Permission) -> Self {
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
