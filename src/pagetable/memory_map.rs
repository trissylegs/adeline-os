use core::ops::Range;
use alloc::{vec::Vec, collections::BTreeSet};
use bitflags::bitflags;
use crate::{STACK_GUARD, println};

use super::{VirtualAddress, PhysicalAddress};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    pub address: VirtualAddress,
    /// The physical address this region maps to. If None this region is identity mapped.
    pub maps_to: Option<PhysicalAddress>,
    pub end: VirtualAddress,
    pub desc: &'static str,
    pub perms: Permission,
}

impl Region {
    pub fn start(&self) -> VirtualAddress {
        self.address
    }

    pub fn end(&self) -> VirtualAddress {
        self.end
    }

    pub fn len(&self) -> VirtualAddress {
        VirtualAddress(self.end.0 - self.address.0)
    }

    pub fn overlaps(&self, other: &Region) -> bool {
        // Checks if two regions overlap
        self.start() < other.end() && self.end() > other.start()
    }
}

#[test_case]
fn test_overlap_true() {
    let a = Region { address:VirtualAddress(0), end:VirtualAddress(10), desc:"a", perms:Permission::R, maps_to: None };
    let b = Region { address: VirtualAddress(5), end: VirtualAddress(15), desc: "b", perms: Permission::R, maps_to: None };
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

// Test overlap returns false when its not overlapping
#[test_case]
fn test_overlap_false() {
    let a = Region { address: VirtualAddress(0), end: VirtualAddress(10), desc: "a", perms: Permission::R, maps_to: None };
    let b = Region { address: VirtualAddress(10), end: VirtualAddress(15), desc: "b", perms: Permission::R, maps_to: None };
    assert!(!a.overlaps(&b));
    assert!(!b.overlaps(&a));
}

pub struct MemoryRegions {
    regions: Vec<Region>
}

impl MemoryRegions {
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    /// Add a new region to the memory map. Returns true if the region was added, false if it overlaps with an existing region.
    /// The memory map is sorted by address.
    ///
    /// If the region is empty (start == end) it is not added.
    ///
    /// # Panics
    /// Panics if the region is invalid (start > end)
    /// # Example
    /// ```
    /// use riscv_os::pagetable::memory_map::{MemoryRegions, Region, VirtualAddress};
    /// use riscv_os::pagetable::memory_map::Permission;
    /// let mut regions = MemoryRegions::new();
    /// assert!(regions.add(0..10, "test", Permission::R));
    /// assert!(regions.add(10..20, "test", Permission::R));
    /// assert!(!regions.add(5..15, "test", Permission::R));
    /// assert!(!regions.add(15..25, "test", Permission::R));
    /// assert!(!regions.add(5..25, "test", Permission::R));
    /// assert!(!regions.add(30..30, "test", Permission::R));
    /// ```
    pub fn add(&mut self, range: Range<u64>, desc: &'static str, perms: Permission) -> bool {
        assert!(range.start <= range.end, "Invalid region: start > end");
        if range.start == range.end {
            return false;
        }
        let region = Region { address: VirtualAddress(range.start), end: VirtualAddress(range.end), desc, perms, maps_to: None };
        if self.regions.iter().any(|r| r.overlaps(&region)) {
            false
        } else {
            self.regions.push(region);
            self.regions.sort_by_key(|r| r.address);
            true
        }
    }

    pub fn add_inital_memory(&mut self, hwinfo: &'static crate::hwinfo::HwInfo, image: &'static crate::linker_info::LinkerInfo) {
        self.add(0..65536, "NULL", Permission::NONE);
        self.add(hwinfo.uart.reg.as_range(), &hwinfo.uart.name, Permission::RW);
        // CLINT is protected by PMP.
        self.add(hwinfo.clint.reg.as_range(), &hwinfo.clint.name, Permission::NONE);
        self.add(hwinfo.plic.reg.as_range(), &hwinfo.plic.name, Permission::RW);
        self.add(hwinfo.rtc.reg.as_range(), &hwinfo.rtc.name, Permission::RW);
        for reserved in hwinfo.reserved_memory.iter() {
            self.add(reserved.as_range(), "Reserved", Permission::NONE);
        }
        self.add(image.text.clone(), "Kernel text", Permission::RX);
        self.add(image.rodata.clone(), "Kernel rodata", Permission::R);
        self.add(image.data.clone(), "Kernel data", Permission::RW);
        let stack_guard = STACK_GUARD.address();
        self.add(image.bss.start .. stack_guard.start, "Kernel bss", Permission::RW);
        self.add(stack_guard.start .. stack_guard.end, "Stack guard", Permission::NONE);
        self.add(stack_guard.end .. image.bss.end, "Kernel stack", Permission::RW);
        self.add(image.tdata.clone(), "Kernel thread template data", Permission::R);
        self.add(image.tbss.clone(), "Kernel thread template bss", Permission::R);

        // Add the kernel heap
        let heap_range = crate::basic_allocator::heap_range();
        self.add(heap_range.as_range(), "Kernel heap", Permission::RW);
    }

    pub fn print(&self) {
        println!("Memory map:");
        for region in self.regions.iter() {
            println!("  {:016x} - {:016x} {:?} {}", region.address.0, region.end.0, region.perms, region.desc);
        }
    }
}

#[test_case]
fn test_add_to_region() {
    let mut regions = MemoryRegions::new();
    assert!(regions.add(0..10, "test", Permission::R));
    assert!(regions.add(10..20, "test", Permission::R));
    assert!(!regions.add(5..15, "test", Permission::R));
    assert!(!regions.add(15..25, "test", Permission::R));
    assert!(regions.add(25..35, "test", Permission::R));
    assert!(!regions.add(5..25, "test", Permission::R));
    assert!(!regions.add(30..30, "test", Permission::R));
}

bitflags! {
    pub struct Permission: u8 {
        #[doc = "No permissions. Used to mark regions that cannot be accessed. Eg; machine mode protected areas"]
        const NONE = 0;

        #[doc = "Readable memory"]
        const R = 0b001;
        #[doc = "Writable memory"]
        const W = 0b010;
        #[doc = "eXecutable memory"]
        const X = 0b100;
        #[doc = "Readable and writable memory"]
        const RW = Self::R.bits | Self::W.bits;
        #[doc = "Readable and executable memory"]
        const RX = Self::R.bits | Self::X.bits;
        #[doc = "Readable, writable and executable memory. WARNING: You shouldn't need this."]
        const RWX = Self::R.bits | Self::W.bits | Self::X.bits;
    }
}

