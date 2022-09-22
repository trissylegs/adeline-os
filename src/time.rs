use core::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, AtomicU64, Ordering},
};

pub use core::time::*;

static TICK_RATE: AtomicU64 = AtomicU64::new(0);
static CLIC_ADDR: AtomicPtr<()> = AtomicPtr::new(null_mut());

pub(crate) fn init_time(hwinfo: &crate::hwinfo::HwInfo) {
    TICK_RATE.store(hwinfo.timebase_freq, Ordering::Relaxed);
}

pub struct Instant {
    tick: u64,
}
