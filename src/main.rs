#![feature(naked_functions)]
#![feature(asm_sym)]
#![feature(asm_const)]
#![feature(default_alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(error_in_core)]
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

use ::time::OffsetDateTime;
use core::{
    arch::asm,
    fmt::{Debug, Write},
    str,
    time::Duration,
};
use fdt_rs::{
    base::DevTree,
    prelude::{FallibleIterator, PropReader},
};
use riscv::register::{
    mtvec,
    scause::{self, Trap},
    sepc, sie, sstatus, stval, stvec,
};
use spin::Mutex;

use crate::{
    console::LockOrDummy,
    isr::{plic, Sip},
    prelude::*,
    sbi::hart::{hsm_extension, HartId},
    time::{sleep, Instant},
};

extern "C" {
    pub static _start_of_data: usize;
    pub static _end_of_data: usize;
    pub static bss_start: usize;
    pub static bss_end: usize;
    pub static stack_limit: usize;
    pub static stack_top: usize;
    // Should be stored in register gp
    pub static __global_pointer: usize;
}

#[no_mangle]
pub extern "C" fn kmain(hart_id: HartId, dtb: *const u8) -> ! {
    sbi::init();
    basic_allocator::init();
    // hwinfo::dump_dtb_hex(dtb);

    let hwinfo = hwinfo::setup_dtb(dtb);

    unsafe {
        plic::init(hwinfo);
        plic::set_threshold(plic::Threshold::Enable);
        // If there's a pending interrupt on uart let's clear it first.
        plic::process_interrupt(hart_id);
    }

    console::init(hwinfo);
    time::init_time(hwinfo);
    time::rtc::init(hwinfo);

    let now = Instant::now();
    println!("now = {:?}", now);

    println!("{:#?}", hwinfo);

    let stvec_addr = trap_entry as *const u8;
    assert_eq!((stvec_addr as usize) & 0b11, 0);

    unsafe {
        stvec::write(stvec_addr as usize, mtvec::TrapMode::Direct);
        let stvec_ret = stvec::read();
        // stvec uses WARL. (Write any values, read legal values)

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

        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        sstatus::set_sie();
    }

    let time = OffsetDateTime::now_utc();
    println!("time: {}", time);

    let sleep_time = Duration::from_secs(1);
    println!("Sleep for {:?}", sleep_time);
    time::sleep(sleep_time);

    let sie_val = sie::read();
    println!("sie       = {:?}", sie_val);
    println!(" .ssoft   = {:?}", sie_val.ssoft());
    println!(" .stimer  = {:?}", sie_val.stimer());
    println!(" .sext    = {:?}", sie_val.sext());
    println!(" .usoft   = {:?}", sie_val.usoft());
    println!(" .utimer  = {:?}", sie_val.utimer());
    println!(" .uext    = {:?}", sie_val.uext());

    println!("heart: {}", hart_id);
    unsafe {
        println!("bss_start: {}", bss_start);
        println!("bss_end: {}", bss_end);
        println!("start_of_memory: {:?}", _start_of_data);
        println!("end_of_memory: {:?}", _end_of_data);
    }
    println!();

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

    /*
    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(example_task()));
    executor.run();
    */

    // shutdown();
    #[allow(unused)]
    loop {
        for b in console::pending_bytes() {
            println!("Got byte: {:02x}", b);
        }

        sleep(Duration::from_millis(200));

        // println!("Suspending!");
        // let suspend = hsm.hart_rentative_suspend(RentativeSuspendType::DEFAULT_RETENTIVE_SUSPEND);
        // println!("Suspend result: {:?}", suspend);
    }
    // shutdown();
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
pub unsafe extern "C" fn stackless_clear_memory(a: *mut u8, b: *mut u8) {
    asm!(
        // Rules:
        // * cannot touch stack or global memory.
        // * must not break s0...s11
        "
        bgeu a0,   a0, 3f
    2:
        sd   zero, 0(a0)
        addi a0,   a0, {reg_size}
        bltu a0,   a1, 2b
    3:
        ret
        ",
        reg_size = const core::mem::size_of::<usize>(),
        options(noreturn)
    );
}

#[naked]
#[no_mangle]
#[link_section = ".text.init"]
pub unsafe extern "C" fn _start(hart_id: usize, dev_tree: *const u8) -> ! {
    asm!(
        // Set global pointer.
        ".option push",
        ".option norelax",
        "la   gp, __global_pointer",
        ".option pop",
        // Setup stack
        "la   sp, stack_top",

        // Save heart_id and device_tree address. So we can call clear_memory
        "mv   s0, a0",
        "mv   s1, a1",

        // stackless_clear_memory(bss_start, bss_end)
        "la   a0, bss_start",
        "la   a1, bss_end",
        "call {clear_memory}",

        // kmain(hart_id, device_tree)
        "mv   a0, s0",             // heart_id: usize
        "mv   a1, s1",             // device_tree: *const u8
        "tail {main}",
        main = sym kmain,
        clear_memory = sym stackless_clear_memory,
        options(noreturn)
    )
}

// #[allow(unsupported_naked_functions)]
#[naked]
#[no_mangle]
// Interrupt handle my be aligned to 2k boundry. So we put it in a specific section and make sure the linker script puts this first.
#[link_section = ".text.trap_entry"]
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
extern "C" fn trap(regs: &mut TrapRegisters) {
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
                time::interrupt_handler(w, regs);
            }
            scause::Interrupt::UserExternal => {
                writeln!(w, "USER EXTERNAL INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::SupervisorExternal => {
                writeln!(w, "SUPERVISOR EXTERNAL INTERRUPT: {:x}", stval);
            }
            scause::Interrupt::Unknown => {
                writeln!(w, "Unknown interupt: {:x}", stval);
            }
        },
        Trap::Exception(ex) => {
            let mut console = unsafe { console::force_unlock() };
            writeln!(console, "*** EXCECPTION ***").ok();
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
            writeln!(console, "regs    = {:#?}", regs).ok();
            let instruction = unsafe { *(sepc as *const u32) };
            writeln!(console, "pc      = 0x{:x}", sepc).ok();
            writeln!(console, "ins     = 0x{:08x}", instruction).ok();

            panic!("Supervisor exception {:?}", ex);
        }
    }
}

static INDENT_STR: &'static str = "                                                                                                                                ";

fn indent(n: usize) -> &'static str {
    INDENT_STR.split_at(n).0
}

fn print_tree<W>(w: &mut W, tree: &DevTree<'_>) -> core::fmt::Result
where
    W: Write + Sized,
{
    let magic = tree.magic();
    let version = tree.version();
    let totalsize = tree.totalsize();

    let boot_cpuid_phys = tree.boot_cpuid_phys();
    let last_comp_version = tree.last_comp_version();
    let off_mem_rsvmap = tree.off_mem_rsvmap();
    let off_dt_struct = tree.off_dt_struct();
    let size_dt_struct = tree.size_dt_struct();
    let off_dt_strings = tree.off_dt_strings();
    let size_dt_strings = tree.off_dt_strings();

    writeln!(w, "DevTree:")?;

    let mut ind = indenter::indented(w);
    ind = ind.with_str(indent(4));
    writeln!(ind, "magic: {magic}")?;
    writeln!(ind, "version: {version}")?;
    writeln!(ind, "totalsize: {totalsize}")?;
    writeln!(ind, "boot_cpuid_phys: {boot_cpuid_phys}")?;
    writeln!(ind, "last_comp_version: {last_comp_version}")?;
    writeln!(ind, "off_mem_rsvmap: {off_mem_rsvmap}")?;
    writeln!(ind, "off_dt_struct: {off_dt_struct}")?;
    writeln!(ind, "size_dt_struct: {size_dt_struct}")?;
    writeln!(ind, "off_dt_strings: {off_dt_strings}")?;
    writeln!(ind, "size_dt_strings: {size_dt_strings}")?;

    writeln!(ind, "reserved_entries:")?;
    ind = ind.with_str(indent(8));
    for re in tree.reserved_entries() {
        let address: u64 = re.address.into();
        let size: u64 = re.size.into();
        writeln!(ind, "fdt_reserve_entry: ")?;
        writeln!(ind, "    address: {address:x}")?;
        writeln!(ind, "    size: {size:x}")?;
    }
    ind = ind.with_str(indent(4));

    writeln!(ind, "nodes:")?;
    ind = ind.with_str(indent(8));

    let mut address_cells = 0;
    let mut size_cells = 0;

    for node in tree.nodes().iterator() {
        if let Ok(node) = node {
            writeln!(ind, "node:")?;
            ind = ind.with_str(indent(12));
            let name = node.name();
            writeln!(ind, "name: {name:?}")?;
            writeln!(ind, "props:")?;
            ind = ind.with_str(indent(16));
            for prop in node.props().iterator() {
                if let Ok(prop) = prop {
                    if let Ok(prop_name) = prop.name() {
                        match prop_name {
                            "reg" if address_cells == 2 && size_cells == 2 => {
                                let address = prop.u64(0).unwrap();
                                let size = prop.u64(1).unwrap();
                                writeln!(ind, "{}: <0x{:x} 0x{:x}>", prop_name, address, size)?;
                            }
                            "reg" if address_cells == 1 && size_cells == 1 => {
                                let address = prop.u32(0).unwrap();
                                let size = prop.u32(1).unwrap();
                                writeln!(ind, "{}: <0x{:x} 0x{:x}>", prop_name, address, size)?;
                            }
                            "reg" if address_cells == 2 || size_cells == 2 => {
                                let value = prop.u64(0).unwrap();
                                writeln!(ind, "{}: <0x{:x}>", prop_name, value)?;
                            }
                            "reg" if address_cells == 1 || size_cells == 1 => {
                                let value = prop.u32(0).unwrap();
                                writeln!(ind, "{}: <0x{:x}>", prop_name, value)?;
                            }
                            "phandle" => {
                                let phandle = prop.phandle(0).unwrap();
                                writeln!(ind, "{prop_name}: <0x{phandle:x}>")?;
                            }
                            "#address-cells" => {
                                let prop_u32 = prop.u32(0).unwrap();
                                address_cells = prop_u32;
                                writeln!(ind, "{prop_name}: <{prop_u32}>")?;
                            }
                            "#size-cells" => {
                                let prop_u32 = prop.u32(0).unwrap();
                                size_cells = prop_u32;
                                writeln!(ind, "{prop_name}: <{prop_u32}>")?;
                            }

                            _ => {
                                if let Ok(prop_str) = prop.str() {
                                    writeln!(
                                        ind,
                                        "{}: {:?} ({})",
                                        prop_name,
                                        prop_str,
                                        prop_str.len()
                                    )?;
                                } else {
                                    writeln!(ind, "{}", prop_name)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        ind = ind.with_str(indent(8));
    }

    Ok(())
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
    // qemu::exit_qemu(qemu::ExitCode::Success);
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
    RenteredCriticalSection,
}

pub static CRITICAL_SECTION_LOCK: Mutex<CritLock> = Mutex::new(CritLock { _opaque: () });

pub struct CritLock {
    _opaque: (),
}
