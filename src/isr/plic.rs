use core::{
    mem::size_of,
    num::NonZeroU32,
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::vec::Vec;
use spin::{Mutex, Once};

use crate::{hwinfo::HwInfo, isr::Sip, println, sbi::hart::HartId};

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
    enable_mutex: Mutex<()>,
}

pub static PLIC: Once<MmioPlic> = Once::INIT;

pub unsafe fn init(hwinfo: &HwInfo) {
    PLIC.call_once(|| (MmioPlic::init(hwinfo)));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InterruptId(pub NonZeroU32);

impl InterruptId {
    pub fn get(self) -> u32 {
        self.0.get()
    }

    fn new(i: u32) -> Option<Self> {
        NonZeroU32::new(i).map(InterruptId)
    }
}

impl From<u32> for InterruptId {
    fn from(n: u32) -> Self {
        InterruptId(NonZeroU32::new(n).expect("interrupt id cannot be 0"))
    }
}

impl MmioPlic {
    unsafe fn init(info: &HwInfo) -> Self {
        // Clear pending interrutps.
        Sip::write(Sip::empty());

        let base = info.plic.reg.base as *mut u8;
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
                enable_mutex: Mutex::new(()),
            };

            for irq in 1..number_of_sources {
                ctx.toggle(irq, false);
                let priority =
                    base.add(PRIORITY_BASE)
                        .add((irq as usize) * PRIORITY_PER_ID) as *mut u32;

                priority.write_volatile(1);
            }
            contexts.push(ctx);
        }

        let plic = Self {
            number_of_sources,
            addr: AtomicPtr::new(base),
            contexts,
        };

        // println!("{:#?}", plic);

        plic
    }

    fn context_for(&self, current_hart: HartId) -> &Context {
        for ctx in &self.contexts {
            if ctx.hart_id == current_hart {
                return ctx;
            }
        }
        panic!("Hart #{} has no context", current_hart.0);
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

    fn set_threshold(&self, arg: Threshold) {
        unsafe {
            let ptr = self
                .hart_base
                .load(Ordering::Relaxed)
                .add(CONTEXT_THRESHOLD);
            ptr.write(arg as u32);
        }
    }

    fn toggle_interrupt(&self, interrupt: InterruptId, enable: bool) {
        let i = interrupt.0.get();
        self.enable_mutex.lock();
        let enable_base = self.enable_base.load(Ordering::Relaxed);
        unsafe {
            let reg = enable_base.add((i as usize) / 32);
            let mask = 1 << (i % 32);

            if enable {
                let old = reg.read_volatile();
                let new = old | mask;
                reg.write_volatile(new);
            } else {
                let old = reg.read_volatile();
                let new = old & !mask;
                reg.write_volatile(new);
            }
        }
    }

    fn claim(&self) -> Option<InterruptId> {
        unsafe {
            let claim_ptr = self.hart_base.load(Ordering::Relaxed).add(CONTEXT_CLAIM);
            let res = claim_ptr.read_volatile();
            InterruptId::new(res)
        }
    }

    pub(crate) fn complete(&self, interrupt: InterruptId) {
        unsafe {
            let complete_ptr = self.hart_base.load(Ordering::Relaxed).add(CONTEXT_CLAIM);
            complete_ptr.write_volatile(interrupt.get());
        }
    }
}

/* unsafe fn read_while_non_zero(ptr: *const u32) -> impl Iterator<Item = InterruptId> {
    core::iter::repeat_with(|| ptr.read_volatile())
        .map(|i| InterruptId::new(i))
        .take_while(|i| i.is_some())
        .map(Option::unwrap)
} */

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Threshold {
    Enable = 0,
    // 1-6 are valid. But we're not using it at this time.
    Disable = 7,
}

pub(crate) fn set_threshold(arg: Threshold) {
    let plic = load_plic();

    for ctx in &plic.contexts {
        ctx.set_threshold(arg);
    }
}

pub(crate) fn enable_interrupt(interrupt: InterruptId) {
    let plic = load_plic();

    for ctx in &plic.contexts {
        ctx.toggle_interrupt(interrupt, true);
    }
}

pub(crate) fn process_interrupt(current_hart: HartId) {
    let plic = load_plic();
    let context = plic.context_for(current_hart);

    if let Some(interrupt) = context.claim() {
        println!("Claimed interrupt {:?}", interrupt);
        // TODO
        context.complete(interrupt);
    }
}

fn load_plic() -> &'static MmioPlic {
    PLIC.get().expect("PLIC not initialized")
}
