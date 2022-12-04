use core::sync::atomic::AtomicBool;
use core::fmt::Write;
use linked_list_allocator::LockedHeap;

use crate::console::sbi_console;

const BASIC_POOL_SIZE: usize = 1024 * 1024;

// Mutable so it get's linked into the correct section. mut keyword may not actually be necessary.
#[cfg(feature = "basic_pool")]
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

pub(crate) fn init_from_free_space(start: *mut u8, end: *const u8) {
    assert!((start as usize) < (end as usize));
    let heap_size = (end as usize) - (start as usize);    
    unsafe {
        writeln!(sbi_console(), "HEAP BYTES: {}", heap_size);
    }

    unsafe {
        let mut heap = HEAP.lock();
        heap.init(start, heap_size);
    }

}

#[cfg(feature = "basic_pool")]
pub(crate) fn init() {    
    if HAS_INIT.swap(true, core::sync::atomic::Ordering::Acquire) {
        return;
    }
    unsafe {
        let (bottom, size) = BASIC_POOL.range();

        let mut heap = HEAP.lock();
        heap.init(bottom as *mut u8, size);
    }
}
