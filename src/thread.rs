use alloc::boxed::Box;
use core::{arch::asm, num::NonZeroUsize, sync::atomic::AtomicUsize};
use memoffset::offset_of;
use riscv::register::sscratch;

use crate::{
    println,
    sbi::{
        hart::{HartId, HartState, Hsm},
        BASE_EXTENSION,
    },
    TrapRegisters,
};

pub type ThreadEntry = alloc::boxed::Box<dyn Fn()>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadId(NonZeroUsize);

// CPU starts with Thread #1. So any thread we start is thread 2 or greater.
pub static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(2);

impl ThreadId {
    pub fn next_thread_id() -> ThreadId {
        // Relaxed because we don't care which order the id's are created in. Just as long as their unique.
        let id = NEXT_THREAD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let id = NonZeroUsize::new(id).expect("NEXT_THREAD_ID == 0");
        ThreadId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ThreadStatus {
    #[default]
    None = 0,
    Suspended,
    Scheduled,
    Running,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadStateMagic(usize);

impl Default for ThreadStateMagic {
    fn default() -> Self {
        Self::VALID
    }
}

impl ThreadStateMagic {
    const VALID: ThreadStateMagic = ThreadStateMagic(0xdeadbeef);
    fn validate(self) {
        if self != Self::VALID {
            panic!("Invalid ThreadStateMagic: {:x}", self.0)
        }
    }
}

#[repr(C)]
pub struct Stack {
    limit: usize,
    top: usize,
}

#[repr(C)]
pub struct ThreadState {
    magic: ThreadStateMagic,
    id: ThreadId,
    status: ThreadStatus,
    entry_point: Option<NonZeroUsize>,
    current_hart_id: HartId,
    stack_limit: usize,
    stack_top: usize,
    registers: TrapRegisters,
}

const STACK_POINTER_OFFSET: usize = offset_of!(ThreadState, stack_top);

pub static _GLOBAL_HART_ENTRY: unsafe extern "C" fn() -> ! = global_hart_entry;

#[naked]
#[no_mangle]
pub unsafe extern "C" fn global_hart_entry() -> ! {
    asm! {
        ".option push",
        ".option norelax",
        "la gp, __global_pointer",
        ".option pop",
        "ld sp, {stack_pointer_offset}(a1)",
        "tail global_hart_entry2",
        stack_pointer_offset = const STACK_POINTER_OFFSET,
        options(noreturn)
    }
}

#[no_mangle]
pub unsafe extern "C" fn global_hart_entry2(hart_id: usize, opaque: usize) -> ! {
    println!("global_hart_entry2({:?}, {:?})", hart_id, opaque);
    // Store current thread in scratch so interrupts can find current thread state.
    sscratch::write(opaque);

    let thread_state = opaque as *mut ThreadState;
    (*thread_state).magic.validate();
    run_thread(HartId(hart_id), &mut *thread_state);
    loop {}
}

pub fn run_thread(hart_id: HartId, thread: &'static mut ThreadState) {
    println!("Thread #{} on Hart #{}", thread.id.0, hart_id.0);
    let hsm = BASE_EXTENSION.get_extension::<Hsm>().unwrap().unwrap();

    loop {
        println!("Suspending Thread #{}", thread.id.0);
        hsm.hart_retentive_suspend(
            crate::sbi::hart::RetentiveSuspendType::DEFAULT_RETENTIVE_SUSPEND,
        );
    }
}

pub fn spawn<F>(hart_id: HartId, f: F)
where
    F: FnOnce(),
    F: Send + 'static,
{
    let boxed = Box::new(f);
}

fn _spawn(hart_id: HartId, f: usize) {
    let hsm = BASE_EXTENSION.get_extension::<Hsm>().unwrap().unwrap();

    let status = hsm
        .hart_get_status(hart_id)
        .unwrap_or_else(|err| panic!("Invalid hart {:?}: {:?}", hart_id, err));

    if status != HartState::Stopped {
        panic!(
            "Cannot spawn on Hart {:?} currently in status: {:?}",
            hart_id, status
        );
    }

    let thread = Box::new(ThreadState {
        magic: ThreadStateMagic::VALID,
        id: ThreadId::next_thread_id(),
        status: ThreadStatus::None,
        entry_point: None,
    });

    todo!()
}
