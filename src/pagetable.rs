//! Implementation of sv39

use core::fmt::{Debug, Formatter};
use const_default::ConstDefault;
use crate::basic_consts::{BITS_2, BITS_26, BITS_9};

pub const PAGE_SIZE: u64 = 4096;
pub const ENTRIES: usize = 512;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum VirtualMemorySystem {
    Sv39,
    Sv48,
    Sv57,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct PhysicalAddr(pub u64);

impl PhysicalAddr {
    pub const fn page_offset(&self) -> u64 {
        self.0 & ((1 << 12) - 1)
    }

    pub const fn ppn0(&self) -> u64 {
        self.0 & (((1 << 9) - 1) << 12) >> 12
    }

    pub const fn ppn1(&self) -> u64 {
        self.0 & (((1 << 9) - 1) << 21) >> 21
    }

    pub const fn ppn2(&self) -> u64 {
        self.0 & (((1 << 26) - 1) << 30) >> 30
    }
}


#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default, ConstDefault)]
pub struct Entry(pub u64);

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.reserved() != 0 {
            write!(f, "{{RES:{:x}}}|", self.reserved())?;
        }
        write!(f, "{:x}", self.ppn2())?;
        write!(f, "|{:x}", self.ppn1())?;
        write!(f, "|{:x}", self.ppn0())?;
        if self.rsw() != 0 {
            write!(f, "|RSW:{:x}", self.rsw())?;
        }
        if self.dirty() { write!(f, "|D")?; }
        if self.accessed() { write!(f, "|A")?; }
        if self.global() { write!(f, "|G")?; }
        if self.user() { write!(f, "|U")?; }
        if self.execute() { write!(f, "|X")?; }
        if self.write() { write!(f, "|W")?; }
        if self.read() { write!(f, "|R")?; }
        if self.valid() { write!(f, "|V")?; }
        Ok(())
    }
}


impl Entry {
    const fn new() -> Self { ConstDefault::DEFAULT }

    const fn get_bit(self, bit: u32) -> bool { (self.0 & (1 << bit)) != 0 }
    pub const fn valid(self) -> bool { self.get_bit(0) }
    pub const fn read(self) -> bool { self.get_bit(1) }
    pub const fn write(self) -> bool { self.get_bit(2) }
    pub const fn execute(self) -> bool { self.get_bit(3) }
    pub const fn user(self) -> bool { self.get_bit(4) }
    pub const fn global(self) -> bool { self.get_bit(5) }
    pub const fn accessed(self) -> bool { self.get_bit(6) }
    pub const fn dirty(self) -> bool { self.get_bit(7) }
    pub const fn rsw(self) -> u8 { ((self.0 >> 8) & BITS_2) as u8 }

    pub const fn ppn0(self) -> u64 {
        (self.0 >> 10) & BITS_9
    }

    pub const fn ppn1(self) -> u64 {
        (self.0 >> 19) & BITS_9
    }

    pub const fn ppn2(self) -> u64 {
        (self.0 >> 28) & BITS_26
    }

    pub const fn reserved(self) -> u64 {
        self.0 >> 54
    }
}

impl Entry {
    pub const fn leaf(self) -> bool {
        !self.read() && !self.write() && !self.execute()
    }

    pub const fn non_leaf(self) -> bool {
        !self.leaf()
    }
}

pub trait Level {
    const PAGE_SIZE: usize;
}
pub trait NonLeaf: Level {
    type Next: Level;
}

struct Level0 {}
impl Level for Level0 {
    const PAGE_SIZE: usize = 4096;
}

struct Level1 {}
impl Level for Level1 {
    const PAGE_SIZE: usize = 1 << 21;
}
impl NonLeaf for Level1 {
    type Next = Level0;
}

struct Level2 {}
impl Level for Level2 {
    const PAGE_SIZE: usize = 1 << 30;
}
impl NonLeaf for Level0 {
    type Next = Level1;
}


#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct PageTable<L: Level> {
    entries: [Entry; ENTRIES],
}

impl<L:NonLeaf> PageTable<L> {
    fn next_level(&self, index: u32) -> Option<PageTable<L::Next>> {
        let e: Entry = self.entries[index];
        if e.valid() & !e.leaf() {

        }
    }
}

impl PageTable<Level2> {

    pub fn print(&self, f: &mut Formatter) {

    }
}


#[cfg(test)]
pub mod test {
    use super::*;

    #[test_case]
    fn page_entry_flags() {
        assert!(Entry(1 << 0).valid());
        assert!(Entry(1 << 1).read());
        assert!(Entry(1 << 2).write());
        assert!(Entry(1 << 3).execute());
        assert!(Entry(1 << 4).user());
        assert!(Entry(1 << 5).global());
        assert!(Entry(1 << 6).accessed());
        assert!(Entry(1 << 7).dirty());
    }

    #[test_case]
    fn page_offset_all1s() {
        assert_eq!(0b111111111111, PhysicalAddr(u64::MAX).page_offset())
    }

    #[test_case]
    fn pp0_all1s() {
        assert_eq!(0b111111111, PhysicalAddr(u64::MAX).ppn0())
    }

    #[test_case]
    fn pp2_all1s() {
        assert_eq!(0b111111111, PhysicalAddr(u64::MAX).ppn1())
    }

    #[test_case]
    fn pp3_all1s() {
        assert_eq!(0b11111111111111111111111111, PhysicalAddr(u64::MAX).ppn2())
    }
}