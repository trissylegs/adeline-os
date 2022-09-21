use core::{
    mem::size_of,
    num::NonZeroU32,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
};

use alloc::vec::Vec;
use spin::{Mutex, Once};

use crate::{hwinfo::HwInfo, sbi::hart::HartId};

pub type Context = u32;

const PLIC_SIZE: usize = 0x10000 / 4;

const PRIORITY_BASE: usize = 0;
const PRIORITY_PER_ID: usize = 4;

const CONTEXT_ENABLE_BASE: usize = 0x2000;
const CONTEXT_ENABLE_SIZE: usize = 0x2000;

const CONTEXT_BASE: usize = 0x200000;
const CONTEXT_SIZE: usize = 0x1000;
const CONTEXT_THRESHOLD: usize = 0x00;
const CONTEXT_CLAIM: usize = 0x04;

const PLIC_DISABLE_THRESHOLD: usize = 0x7;
const PLIC_ENABLE_THRESHOLD: usize = 0x0;

pub struct MmioPlic {
    enable_locks: Vec<Mutex<()>>,
    addr: AtomicPtr<u8>,
}

pub static PLIC: Once<Mutex<MmioPlic>> = Once::INIT;

pub unsafe fn init(hwinfo: &HwInfo) {
    PLIC.call_once(|| Mutex::new(MmioPlic::new(hwinfo)));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InterruptId(pub NonZeroU32);

impl InterruptId {
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl From<u32> for InterruptId {
    fn from(n: u32) -> Self {
        InterruptId(NonZeroU32::new(n).expect("interrupt id cannot be 0"))
    }
}

impl MmioPlic {
    pub unsafe fn new(info: &HwInfo) -> Self {
        // Usually 0x0C00_0000
        // Comments will assme this address in future. To match SiFive's documetnations.
        let mut enable_locks = Vec::with_capacity(info.harts.len());
        for _ in &info.harts {
            enable_locks.push(Mutex::new(()));
        }

        Self {
            enable_locks,
            addr: AtomicPtr::new(info.plic.reg.base as *mut _),
        }
    }
    /*
       pub fn toggle(&self, irq: InterruptId, enable: bool) {
           let _lock = self.enable_locks.lock();
           let hwirq = irq.get();

           unsafe {
               let reg = self
                   .addr
                   .load(Ordering::Relaxed)
                   .add((hwirq as usize / 32) * size_of::<u32>()) as *mut u32;
               let mask = 1 << (hwirq % 32);

               if enable {
                   reg.write_volatile(reg.read_volatile() | mask);
               } else {
                   reg.write_volatile(reg.read_volatile() & !mask);
               }
           }

           drop(_lock);
       }
    */

    pub fn context_for(&self, hart: HartId) -> Context {
        // Assming S-mode
        u32::try_from(hart.0).expect("hartid too big") * 2 + 1
    }

    pub fn priority(&self, source: u32) -> &AtomicU32 {
        if source < 1 || source > 511 {
            panic!("Invalid interrupt source: {}", source);
        }
        unsafe {
            let addr = self
                .addr
                .load(Ordering::Relaxed)
                .add((source as usize) * size_of::<u32>()) as *mut AtomicU32;

            &*addr
        }
    }

    fn pending_array(&self) -> &[AtomicU32; 0x14] {
        unsafe {
            let addr = self.addr.load(Ordering::Relaxed).add(0x1000) as *mut [AtomicU32; 0x14];
            &*addr
        }
    }

    fn pending(&self, source: u32) -> bool {
        let arr = self.pending_array();
        let source = source as usize;
        arr[source / 32].load(Ordering::Relaxed) & (1 << (source % 32)) != 0
    }

    fn enables(&self, hart: HartId) -> &[AtomicU32; 0x14] {
        let base_addr = self.addr.load(Ordering::Relaxed);
        let hart = hart.0;

        unsafe {
            let addr = base_addr.add(0x2000 + (0x80 * hart) + 0x80) as *mut _;
            &*addr
        }
    }

    fn toggle_irq(&self, hart: HartId, irq: InterruptId, enable: bool) {
        // Because we have to read-modify-write a register we a lock to esnure it happens all at once.
        let lock = self.enable_locks[hart.0 as usize].lock();
        let irq = irq.get();
        let reg = &self.enables(hart)[(irq as usize) / 32];
        let mask = 1 << (irq % 32);

        let read = reg.load(Ordering::Relaxed);
        if enable {
            reg.store(read | mask, Ordering::Relaxed);
        } else {
            reg.store(read & !mask, Ordering::Relaxed);
        }

        drop(lock);
    }

    fn priority_threshold(&self, _hart: HartId) -> &AtomicU32 {
        let base = self.addr.load(Ordering::Relaxed);

        unsafe {
            let _addr = base.add(20);
        }
        todo!()
    }
}
