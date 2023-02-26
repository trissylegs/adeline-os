#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(custom_test_frameworks)]
#![feature(never_type)]
#![feature(error_in_core)]
#![feature(fn_align)]
#![feature(type_alias_impl_trait)]
#![feature(int_roundings)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(dead_code)]
#![no_std]
#![no_main]

extern crate alloc;

mod prelude;

mod asm;
mod basic_allocator;
mod basic_consts;
mod console;
mod hwinfo;
mod io;
mod isr;
mod linker_info;
mod pagetable;
mod panic;
mod sbi;
mod task;
mod time;
mod trap;
mod util;

use const_default::ConstDefault;
use hwinfo::DtbRef;
use pagetable::{PageTable};
use ::time::OffsetDateTime;
use core::{
    cell::UnsafeCell,
    sync::atomic::AtomicBool,
    time::Duration,
};

use riscv::register::{
    mtvec,
     sie, sstatus,  stvec, satp,
};
use spin::Mutex;

use crate::{
    isr::plic,
    prelude::*,
    sbi::{
        hart::{hsm_extension, HartId},
        reset::shutdown,
    },
    time::{sleep, Instant},
    linker_info::{__stack_limit, __stack_top, __global_pointer, __image_end}, pagetable::{place_dump_map, BigPage},
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
pub extern "C" fn kmain(hart_id: HartId, dtb: DtbRef) -> ! {
    unsafe {
        STACK_GUARD.init();
    }

    let has_booted = BOOTLOOP_DETECT.swap(true, core::sync::atomic::Ordering::SeqCst);
    if has_booted {
        panic!("Boot loop detected");
    }

    sbi::init();
    unsafe {
        basic_allocator::init_from_free_space(&mut __image_end as *mut u8 as *mut u8, &dtb);
    }

    let hwinfo = hwinfo::setup_dtb(dtb);
    unsafe {
        basic_allocator::finish_init(hwinfo);
    }


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
    // println!(    "fdt:      {:08x} - {:08x}", hwinfo.tree_range.start, hwinfo.tree_range.end);

    let now = Instant::now();
    println!("now = {:?}", now);

    println!("{:#?}", hwinfo);

    let stvec_addr = asm::trap_entry as *const u8;
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
        place_dump_map(&mut *pt);
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

    for mr in hwinfo.memory_layout() {
        println!("{:?}", mr);

        let mut level_start = BigPage::Page(0);
        let mut prev_level = pagetable::PageLevel::Level0;
        let mut level_count = 0;

        for p in mr.big_pages() {
            let level = p.level();
            if prev_level != level {
                if level_count > 0 {
                    println!("  {} ({} times)", level_start, level_count);
                    level_count = 0;
                }
                level_start = p;
                prev_level = level;
            }
            level_count += 1;
        }
        if level_count > 0 {
            println!("  {} ({} times)", level_start, level_count);
        }
    }

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
