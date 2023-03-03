use core::sync::atomic::{AtomicBool, Ordering};
use core::fmt::Write;
use linked_list_allocator::LockedHeap;

use crate::console::sbi_console;
use crate::hwinfo::{PhysicalAddressRange, PhysicalAddressKind, HwInfo, DtbRef};

const BASIC_POOL_SIZE: usize = 1024 * 1024;

// Mutable so it get's linked into the correct section. mut keyword may not actually be necessary.

static mut BASIC_POOL: BasicPoolMemory = BasicPoolMemory::new();
static HAS_INIT: AtomicBool = AtomicBool::new(false);

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[repr(align(4096))]
struct BasicPoolMemory {
    pool: [u8; BASIC_POOL_SIZE],
}

impl BasicPoolMemory {
    const fn new() -> BasicPoolMemory {
        BasicPoolMemory {
            pool: [0; BASIC_POOL_SIZE],
        }
    }

    fn range(&self) -> (usize, usize) {
        (&self.pool[0] as *const _ as usize, BASIC_POOL_SIZE)
    }
}

pub(crate) unsafe fn init_from_free_space(start: *mut u8, end: &DtbRef) {
    assert!((start as usize) < (end.start() as usize));
    let heap_size = (end.start() as usize) - (start as usize);
    unsafe {
        writeln!(sbi_console(), "HEAP BYTES: {}", heap_size).ok();
    }
    let mut heap = HEAP.lock();
    heap.init(start, heap_size);
}

pub fn heap_range() -> PhysicalAddressRange {
    let heap = HEAP.lock();
    let start = heap.bottom() as u64;
    let end = heap.top() as u64;
    PhysicalAddressRange::new(start..end, PhysicalAddressKind::Writable, "heap".into())
}

pub(crate) unsafe fn finish_init(hwinfo: &HwInfo) {
    let ram = &hwinfo.ram[0];
    let end_of_ram = ram.end;
    let mut heap = HEAP.lock();
    let top = heap.top() as u64;
    if top < end_of_ram {
        heap.extend((end_of_ram - top) as usize);
    }
}

pub(crate) fn init() {
    if HAS_INIT.swap(true, Ordering::Acquire) {
        return;
    }
    unsafe {
        let (bottom, size) = BASIC_POOL.range();

        let mut heap = HEAP.lock();
        heap.init(bottom as *mut u8, size);
    }
}
