#![feature(naked_functions)]
#![feature(asm_sym)]
#![feature(asm_const)]
#![feature(default_alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(error_in_core)]
#![feature(fn_align)]
#![feature(type_alias_impl_trait)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(dead_code)]
#![no_std]
#![no_main]

extern crate alloc;

mod prelude;

mod basic_allocator;
mod console;
mod hwinfo;
mod io;
mod isr;
mod pagetable;
mod panic;
mod sbi;
mod task;
mod time;
mod util;
mod basic_consts;
mod linker_info;

use const_default::ConstDefault;
use pagetable::{PageTable};
use ::time::OffsetDateTime;
use core::{
    arch::asm,
    cell::UnsafeCell,
    fmt::{Debug, Write},
    sync::atomic::AtomicBool,
    time::Duration,
};

use riscv::register::{
    mtvec,
    scause::{self, Trap},
    sepc, sie, sstatus, stval, stvec, satp,
};
use spin::Mutex;

use crate::{
    console::LockOrDummy,
    hwinfo::{MmioRegions, MemoryRegions, ReservedRegions},
    isr::{plic, Sip},
    prelude::*,
    sbi::{
        hart::{hsm_extension, HartId},
        reset::shutdown,
    },
    time::{sleep, Instant},
    linker_info::{__bss_start, __stack_limit, __stack_top, __global_pointer}, pagetable::dumb_map,
};

#[repr(align(4096))]
pub struct StackGuardPage {
    bytes: UnsafeCell<[u64; 512]>,
}
unsafe impl Sync for StackGuardPage {}

impl StackGuardPage {
    unsafe fn init(&self) {
        let bytes = self.bytes.get();
        for word in (*bytes).iter_mut() {
            *word = 0x3355335533553355
        }
    }

    pub(crate) fn check(&self) {
        unsafe {
            let byte = self.bytes.get();
            assert_eq!((*byte)[511], 0x3355335533553355, "Stack guard corrupted");
        }
    }
}

#[link_section = ".stack_guard"]
#[no_mangle]
pub static STACK_GUARD: StackGuardPage = StackGuardPage {
    bytes: UnsafeCell::new([0; 512]),
};

static BOOTLOOP_DETECT: AtomicBool = AtomicBool::new(false);

static WIP_PAGETABLE: Mutex<PageTable> = Mutex::new(PageTable::DEFAULT);

#[no_mangle]
pub extern "C" fn kmain(hart_id: HartId, dtb: *const u8) -> ! {
    unsafe {
        STACK_GUARD.init();
    }

    let has_booted = BOOTLOOP_DETECT.swap(true, core::sync::atomic::Ordering::SeqCst);
    if has_booted {
        panic!("Boot loop detected");
    }

    sbi::init();
    basic_allocator::init();

    let hwinfo = hwinfo::setup_dtb(dtb);
    STACK_GUARD.check();

    unsafe {
        plic::init(hwinfo);
        plic::set_threshold(plic::Threshold::Enable);
        // If there's a pending interrupt on uart let's clear it first.
        plic::process_interrupt(hart_id);
    }

    console::init(hwinfo);
    time::init_time(hwinfo);
    time::rtc::init(hwinfo);

    linker_info::print_address_ranges();
    println!(    " fdt:     {:08x} - {:08x}", hwinfo.tree_range.start, hwinfo.tree_range.end);

    for mmio in hwinfo.get_mmio_regions() {
        println!("mmio:     {:08x} - {:08x}", mmio.start, mmio.end);
    }

    for reserved in hwinfo.get_reserved_regions() {
        println!("reserved: {:08x} - {:08x}", reserved.start, reserved.end);
    }

    for mem in hwinfo.get_memory_regions() {
        println!("memory:   {:08x} - {:08x}", mem.start, mem.end);
    }

    let now = Instant::now();
    println!("now = {:?}", now);

    println!("{:#?}", hwinfo);

    let stvec_addr = trap_entry as *const u8;
    assert_eq!((stvec_addr as usize) & 0b11, 0);

    let stvec_ret = unsafe {
        stvec::write(stvec_addr as usize, mtvec::TrapMode::Direct);
        // stvec uses WARL. (Write any values, read legal values)
        stvec::read()
    };


    println!(
        "stvec address: Wrote: {:?}. Read: {:?}",
        stvec_addr,
        stvec_ret.address() as *const u8
    );

    println!(
        "stvec wrote:   Wrote: {:?}. Read: {:?}",
        mtvec::TrapMode::Direct,
        stvec_ret.trap_mode()
    );

    unsafe {

        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        sstatus::set_sie();
    }

    let time = OffsetDateTime::now_utc();
    println!("time: {}", time);

    let sie_val = sie::read();
    println!("sie       = {:?}", sie_val);
    println!(" .ssoft   = {:?}", sie_val.ssoft());
    println!(" .stimer  = {:?}", sie_val.stimer());
    println!(" .sext    = {:?}", sie_val.sext());
    println!(" .usoft   = {:?}", sie_val.usoft());
    println!(" .utimer  = {:?}", sie_val.utimer());
    println!(" .uext    = {:?}", sie_val.uext());

    println!("heart: {}", hart_id);
    println!();

    pagetable::print_current_page_table();

    {
        let mut pt = WIP_PAGETABLE.lock();
        *pt = dumb_map();
        println!("{:?}", *pt);

        let root_addr = (&*pt) as *const PageTable as u64;
        // Update page table
        let pa = pagetable::PhysicalAddress(root_addr);
        let ppn = pa.ppn();

        unsafe {
            satp::set(satp::Mode::Sv48, 1, ppn as usize);
        }
    };

    pagetable::print_current_page_table();

    let hsm = hsm_extension();

    for hart in &hwinfo.harts {
        let status = hsm.hart_get_status(hart.hart_id);
        match status {
            Ok(status) => println!("{:?}: {:?}", hart.hart_id, status),
            Err(err) => println!("{:?} invalid: ({:?})", hart.hart_id, err),
        }
    }

    #[cfg(test)]
    test_main();

    // shutdown();
    #[allow(unused)]
    let mut do_shutdown = false;
    while !do_shutdown {
        for b in console::pending_bytes() {
            println!("Got byte: {:02x}", b);
            if b == 0x03 {
                do_shutdown = true;
            }
        }

        if !do_shutdown {
            sleep(Duration::from_millis(200));
        }

        // println!("Suspending!");
        // let suspend = hsm.hart_retentive_suspend(RetentiveSuspendType::DEFAULT_RETENTIVE_SUSPEND);
        // println!("Suspend result: {:?}", suspend);
    }
    shutdown();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

#[repr(C)]
struct TrapRegisters {
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
}

impl Debug for TrapRegisters {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapRegisters")
            .field("ra", &format_args!("0x{:>08x}", &self.ra))
            .field("sp", &format_args!("0x{:>08x}", &self.sp))
            .field("gp", &format_args!("0x{:>08x}", &self.gp))
            .field("tp", &format_args!("0x{:>08x}", &self.tp))
            .field("t0", &format_args!("0x{:>08x}", &self.t0))
            .field("t1", &format_args!("0x{:>08x}", &self.t1))
            .field("t2", &format_args!("0x{:>08x}", &self.t2))
            .field("s0", &format_args!("0x{:>08x}", &self.s0))
            .field("s1", &format_args!("0x{:>08x}", &self.s1))
            .field("a0", &format_args!("0x{:>08x}", &self.a0))
            .field("a1", &format_args!("0x{:>08x}", &self.a1))
            .field("a2", &format_args!("0x{:>08x}", &self.a2))
            .field("a3", &format_args!("0x{:>08x}", &self.a3))
            .field("a4", &format_args!("0x{:>08x}", &self.a4))
            .field("a5", &format_args!("0x{:>08x}", &self.a5))
            .field("a6", &format_args!("0x{:>08x}", &self.a6))
            .field("a7", &format_args!("0x{:>08x}", &self.a7))
            .field("s2", &format_args!("0x{:>08x}", &self.s2))
            .field("s3", &format_args!("0x{:>08x}", &self.s3))
            .field("s4", &format_args!("0x{:>08x}", &self.s4))
            .field("s5", &format_args!("0x{:>08x}", &self.s5))
            .field("s6", &format_args!("0x{:>08x}", &self.s6))
            .field("s7", &format_args!("0x{:>08x}", &self.s7))
            .field("s8", &format_args!("0x{:>08x}", &self.s8))
            .field("s9", &format_args!("0x{:>08x}", &self.s9))
            .field("s10", &format_args!("0x{:>08x}", &self.s10))
            .field("s11", &format_args!("0x{:>08x}", &self.s11))
            .field("t3", &format_args!("0x{:>08x}", &self.t3))
            .field("t4", &format_args!("0x{:>08x}", &self.t4))
            .field("t5", &format_args!("0x{:>08x}", &self.t5))
            .field("t6", &format_args!("0x{:>08x}", &self.t6))
            .finish()
    }
}

// Do a memset ensuring we don't use any stack memory.

#[naked]
#[no_mangle]
#[link_section = ".text.init"]
pub unsafe extern "C" fn _start(hart_id: usize, dev_tree: *const u8) -> ! {
    asm!(
        // Set global pointer.
        ".option push",
        ".option norelax",
        "la   gp, {global_pointer}",
        ".option pop",
        // Setup stack
        "la   sp, {stack_top}",

        // Save heart_id and device_tree address. So we can call clear_memory
        "mv   s0, a0",
        "mv   s1, a1",

        // memset(bss_start, 0, stack_limit - bss_start);
        "la   a0, {bss_start}",
        "li   a1, 0",
        "la   a2, {stack_limit}",
        "sub  a2, a2, a0",
        "call memset",

        // kmain(hart_id, device_tree)
        "mv   a0, s0",             // heart_id: usize
        "mv   a1, s1",             // device_tree: *const u8
        "tail {kmain}",
        global_pointer = sym __global_pointer,
        stack_top = sym __stack_top,
        bss_start = sym __bss_start,
        stack_limit = sym __stack_limit,
        kmain = sym kmain,
        options(noreturn)
    )
}

#[naked]
#[no_mangle]
// Interrupt CSR uses lowest bits for flags so handler must be aligned to 2048 bytes.
#[repr(align(4096))]
#[cfg(target_pointer_width = "64")]
pub unsafe extern "C" fn trap_entry() {
    asm!(
        "addi  sp, sp, -31 * 8", /* Allocate stack space */
        "sd    ra,  0 * 8(sp)",  /* Push registers */
        "sd    sp,  1 * 8(sp)", /* fixme: this is saving the updated value of sp. Not it's value *before* the trap was called. */
        "sd    gp,  2 * 8(sp)",
        "sd    tp,  3 * 8(sp)",
        "sd    t0,  4 * 8(sp)",
        "sd    t1,  5 * 8(sp)",
        "sd    t2,  6 * 8(sp)",
        "sd    s0,  7 * 8(sp)",
        "sd    s1,  8 * 8(sp)",
        "sd    a0,  9 * 8(sp)",
        "sd    a1, 10 * 8(sp)",
        "sd    a2, 11 * 8(sp)",
        "sd    a3, 12 * 8(sp)",
        "sd    a4, 13 * 8(sp)",
        "sd    a5, 14 * 8(sp)",
        "sd    a6, 15 * 8(sp)",
        "sd    a7, 16 * 8(sp)",
        "sd    s2, 17 * 8(sp)",
        "sd    s3, 18 * 8(sp)",
        "sd    s4, 19 * 8(sp)",
        "sd    s5, 20 * 8(sp)",
        "sd    s6, 21 * 8(sp)",
        "sd    s7, 22 * 8(sp)",
        "sd    s8, 23 * 8(sp)",
        "sd    s9, 24 * 8(sp)",
        "sd   s10, 25 * 8(sp)",
        "sd   s11, 26 * 8(sp)",
        "sd    t3, 27 * 8(sp)",
        "sd    t4, 28 * 8(sp)",
        "sd    t5, 29 * 8(sp)",
        "sd    t6, 30 * 8(sp)",
        "mv    a0, sp",
        "call trap",
        /* Pop registers */
        "ld    ra,  0 * 8(sp)", /* Push registers */
        "ld    sp,  1 * 8(sp)", /* fixme: this is saving the updated value of sp. Not it's value *before* the trap was called. */
        "ld    gp,  2 * 8(sp)",
        "ld    tp,  3 * 8(sp)",
        "ld    t0,  4 * 8(sp)",
        "ld    t1,  5 * 8(sp)",
        "ld    t2,  6 * 8(sp)",
        "ld    s0,  7 * 8(sp)",
        "ld    s1,  8 * 8(sp)",
        "ld    a0,  9 * 8(sp)",
        "ld    a1, 10 * 8(sp)",
        "ld    a2, 11 * 8(sp)",
        "ld    a3, 12 * 8(sp)",
        "ld    a4, 13 * 8(sp)",
        "ld    a5, 14 * 8(sp)",
        "ld    a6, 15 * 8(sp)",
        "ld    a7, 16 * 8(sp)",
        "ld    s2, 17 * 8(sp)",
        "ld    s3, 18 * 8(sp)",
        "ld    s4, 19 * 8(sp)",
        "ld    s5, 20 * 8(sp)",
        "ld    s6, 21 * 8(sp)",
        "ld    s7, 22 * 8(sp)",
        "ld    s8, 23 * 8(sp)",
        "ld    s9, 24 * 8(sp)",
        "ld   s10, 25 * 8(sp)",
        "ld   s11, 26 * 8(sp)",
        "ld    t3, 27 * 8(sp)",
        "ld    t4, 28 * 8(sp)",
        "ld    t5, 29 * 8(sp)",
        "ld    t6, 30 * 8(sp)",
        "addi  sp, sp, 31 * 8", /* Deallocate stack space */
        "sret",
        options(noreturn)
    );
}

#[no_mangle]
#[allow(unused_must_use)]
extern "C" fn trap(registers: &mut TrapRegisters) {
    let sepc = sepc::read();
    let sstatus = sstatus::read();
    let sie = sie::read();
    let sip = Sip::read();
    let scause = scause::read();
    let stval = stval::read();

    let mut w = LockOrDummy::Dummy;

    writeln!(w, "sepc: {:?}", sepc);
    writeln!(w, "sstatus: {:?}", sstatus);
    writeln!(w, "sie: {:?}", sie);
    writeln!(w, "sip: {:?}", sip);
    writeln!(w, "scause: {:?}", scause.cause());
    writeln!(w, "stval: {:?}", stval);

    match scause.cause() {
        Trap::Interrupt(int) => match int {
            scause::Interrupt::UserSoft => {
                writeln!(w, "USER SOFTWARE INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::SupervisorSoft => {
                writeln!(w, "SUPERVISOR SOFTWARE INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::UserTimer => {
                writeln!(w, "USER TIMER: {:x}", stval);
            }
            scause::Interrupt::SupervisorTimer => {
                time::interrupt_handler(w, registers);
            }
            scause::Interrupt::UserExternal => {
                writeln!(w, "USER EXTERNAL INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::SupervisorExternal => {
                writeln!(w, "SUPERVISOR EXTERNAL INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::Unknown => {
                writeln!(w, "Unknown interrupt: {:x}", stval);
            }
        },
        Trap::Exception(ex) => {
            let mut console = unsafe { console::force_unlock() };
            writeln!(console, "*** EXCEPTION ***").ok();
            writeln!(console, "sepc    = 0x{:x}", sepc).ok();
            writeln!(console, "sstatus = {:?}", sstatus).ok();
            writeln!(console, " .sie   = {:?}", sstatus.sie()).ok();
            writeln!(console, " .spie  = {:?}", sstatus.spie()).ok();
            writeln!(console, " .spp   = {:?}", sstatus.spp()).ok();
            writeln!(console, " .uie   = {:?}", sstatus.uie()).ok();
            writeln!(console, " .upie  = {:?}", sstatus.upie()).ok();
            writeln!(console, " .fs    = {:?}", sstatus.fs()).ok();
            writeln!(console, " .xs    = {:?}", sstatus.xs()).ok();
            writeln!(console, "sie     = {:?}", sie).ok();
            writeln!(console, "scause  = 0x{:x}", scause.bits()).ok();
            writeln!(console, " .code  = {:?}", scause.code()).ok();
            writeln!(console, " .cause = {:?}", scause.cause()).ok();
            writeln!(console, "stval   = 0x{:x}", stval).ok();
            writeln!(console, "registers:").ok();
            writeln!(console, "  ra    = 0x{:x}", registers.ra);
            writeln!(console, "  sp    = 0x{:x}", registers.sp);
            writeln!(console, "  gp    = 0x{:x}", registers.gp);
            writeln!(console, "  tp    = 0x{:x}", registers.tp);
            writeln!(console, "  t0    = 0x{:x}", registers.t0);
            writeln!(console, "  t1    = 0x{:x}", registers.t1);
            writeln!(console, "  t2    = 0x{:x}", registers.t2);
            writeln!(console, "  s0    = 0x{:x}", registers.s0);
            writeln!(console, "  s1    = 0x{:x}", registers.s1);
            writeln!(console, "  a0    = 0x{:x}", registers.a0);
            writeln!(console, "  a1    = 0x{:x}", registers.a1);
            writeln!(console, "  a2    = 0x{:x}", registers.a2);
            writeln!(console, "  a3    = 0x{:x}", registers.a3);
            writeln!(console, "  a4    = 0x{:x}", registers.a4);
            writeln!(console, "  a5    = 0x{:x}", registers.a5);
            writeln!(console, "  a6    = 0x{:x}", registers.a6);
            writeln!(console, "  a7    = 0x{:x}", registers.a7);
            writeln!(console, "  s2    = 0x{:x}", registers.s2);
            writeln!(console, "  s3    = 0x{:x}", registers.s3);
            writeln!(console, "  s4    = 0x{:x}", registers.s4);
            writeln!(console, "  s5    = 0x{:x}", registers.s5);
            writeln!(console, "  s6    = 0x{:x}", registers.s6);
            writeln!(console, "  s7    = 0x{:x}", registers.s7);
            writeln!(console, "  s8    = 0x{:x}", registers.s8);
            writeln!(console, "  s9    = 0x{:x}", registers.s9);
            writeln!(console, "  s10   = 0x{:x}", registers.s10);
            writeln!(console, "  s11   = 0x{:x}", registers.s11);
            writeln!(console, "  t3    = 0x{:x}", registers.t3);
            writeln!(console, "  t4    = 0x{:x}", registers.t4);
            writeln!(console, "  t5    = 0x{:x}", registers.t5);
            writeln!(console, "  t6    = 0x{:x}", registers.t6);

            let instruction = unsafe { *(sepc as *const u32) };
            writeln!(console, "pc      = 0x{:x}", sepc).ok();
            writeln!(console, "ins     = 0x{:08x}", instruction).ok();

            panic!("Supervisor exception {:?}", ex);
        }
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) -> () {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    shutdown();
}

#[test_case]
fn hello_world() {
    println!("Hello world!");
}

#[macro_export]
macro_rules! wait_for {
    ($cond:expr) => {
        while !$cond {
            core::hint::spin_loop()
        }
    };
}

pub enum CriticalSectionError {
    ReenteredCriticalSection,
}

pub static CRITICAL_SECTION_LOCK: Mutex<CritLock> = Mutex::new(CritLock { _opaque: () });

pub struct CritLock {
    _opaque: (),
}
