use core::{
    mem::size_of,
    num::NonZeroU32,
    str,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
};

use alloc::vec::Vec;
use spin::{Mutex, Once};

use crate::{hwinfo::HwInfo, println, sbi::hart::HartId};

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

#[derive(Debug)]
pub struct MmioPlic {
    addr: AtomicPtr<u8>,
    contexts: Vec<Context>,
    number_of_sources: u32,
}

#[derive(Debug)]
pub struct Context {
    index: usize,
    hart_id: HartId,
    hart_base: AtomicPtr<u32>,
    enable_base: AtomicPtr<u32>,
}

pub static PLIC: Once<Mutex<MmioPlic>> = Once::INIT;

pub unsafe fn init(hwinfo: &HwInfo) {
    PLIC.call_once(|| Mutex::new(MmioPlic::init(hwinfo)));
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
    unsafe fn init(info: &HwInfo) -> Self {
        let mut base = info.plic.reg.base as *mut u8;
        let number_of_sources = info.plic.number_of_sources;

        let mut contexts = Vec::with_capacity(info.plic.contexts.len());

        for ctx in &info.plic.contexts {
            let index = ctx.index;
            let hart_id = ctx.hart_id;
            let hart_base =
                AtomicPtr::new(base.add(CONTEXT_BASE).add(CONTEXT_SIZE * ctx.index) as *mut u32);
            let enable_base = AtomicPtr::new(
                base.add(CONTEXT_ENABLE_BASE)
                    .add(CONTEXT_ENABLE_SIZE * ctx.index) as *mut u32,
            );

            let mut ctx = Context {
                index,
                hart_id,
                hart_base,
                enable_base,
            };

            for irq in 1..number_of_sources {
                ctx.toggle(irq, false);
                let priority = base
                    .add(PRIORITY_BASE)
                    .add((irq as usize) * PRIORITY_PER_ID);

                priority.write_volatile(1);
            }
        }

        let mut plic = Self {
            number_of_sources,
            addr: AtomicPtr::new(base),
            contexts,
        };

        println!("{:#?}", plic);

        plic
    }
}

impl Context {
    fn toggle(&mut self, irq: u32, enable: bool) {
        unsafe {
            let reg = self
                .enable_base
                .load(Ordering::Relaxed)
                .add((irq as usize / 32) * size_of::<u32>());
            let mask = 1 << (irq % 32);

            if enable {
                reg.write_volatile(reg.read_volatile() | mask);
            } else {
                reg.write_volatile(reg.read_volatile() & !mask);
            }
        }
    }
}
