use alloc::collections::BTreeMap;
use core::cmp::Ordering;
use core::fmt::Debug;
use core::ops::Range;

use crate::hwinfo::{IommuRegions, MemoryRegions, ReservedRegions};

#[derive(Debug)]
pub enum RegionKind {
    /// Memory region is not present. Or no information is known.
    /// Should remain unmapped
    None,
    /// Memory region is reserved by SBI.
    /// Should reamin unmapped
    Reserved,
    /// Memory mapped IO region. Used for communiating with hardware.
    /// Should be mapped as read-writable with no caching.
    Mmio,
    /// Memory contains executable kernel could.
    /// Should be mapped as executable read-only.
    Executable,
    /// Memory contains kernel read-only sections.
    Readonly,
    /// Memory contains writable kernel memory or unused free memory.
    Writable,
}

pub struct MemoryLayout {
    pub(crate) regions: BTreeMap<MemoryRange, RegionKind>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MemoryRange {
    pub start: usize,
    pub end: usize,
}

impl MemoryRange {
    #[allow(unreachable_code)]
    pub(crate) fn subtract_overlap(
        &self,
        other: &MemoryRange,
    ) -> (Option<MemoryRange>, Option<MemoryRange>) {
        if self.end < other.start {
            // | self   |
            //             | other |
            // | result |
            (Some(*self), None)
        } else if other.end <= self.start {
            //            | self   |
            // | other |
            //            | result |
            (Some(*self), None)
        } else if self.start == other.start && self.end > other.end {
            // | self           |
            // | other |
            //         | result |
            (
                Some(MemoryRange {
                    start: other.end,
                    end: self.end,
                }),
                None,
            )
        } else if self.start < other.start && self.end > other.end {
            // | self         |
            //          | other  |
            // | result |
            (
                Some(MemoryRange {
                    start: self.start,
                    end: other.start,
                }),
                None,
            )
        } else if self.start > other.start && self.start < other.end && self.end > other.end {
            //      | self      |
            // | other |
            //         | result |
            (
                Some(MemoryRange {
                    start: other.end,
                    end: self.end,
                }),
                None,
            )
        } else {
            todo!("self: {:?}, other: {:?}", self, other)
        }
    }
}

impl Debug for MemoryRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:x}..{:x}", self.start, self.end)
    }
}

impl PartialOrd for MemoryRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Range<usize>> for MemoryRange {
    fn from(r: Range<usize>) -> Self {
        MemoryRange {
            start: r.start,
            end: r.end,
        }
    }
}

impl Ord for MemoryRange {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering::*;
        if self.start < other.start {
            if self.end > other.start {
                Equal
            } else {
                Less
            }
        } else if self.start == other.start {
            Equal
        } else if self.start < other.end {
            Equal
        } else {
            Greater
        }
    }
}

impl MemoryLayout {
    pub(crate) fn new(hwinfo: &'static crate::hwinfo::HwInfo) -> Self {
        let mut regions = BTreeMap::new();

        for mmio in hwinfo.get_mmio_regions() {
            regions.insert(mmio.into(), RegionKind::Mmio);
        }

        for res in hwinfo.get_reserved_regions() {
            regions.insert(res.into(), RegionKind::Reserved);
        }

        for mem in hwinfo.get_memory_regions() {
            let mut _mem = MemoryRange::from(mem);
            // Todo
        }

        MemoryLayout { regions }
    }
}
