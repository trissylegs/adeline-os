
#![feature(naked_functions)]
#![feature(default_alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![allow(dead_code)]
#![no_std]
#![no_main]

extern crate alloc;

mod io;
mod pagetable;
mod sbi;
mod uart;
mod traits;
mod basic_allocator;
// mod dev_tree;

use riscv::register::scause::Trap;
use sbi::*;

use core::{fmt::{Write, Debug}, arch::asm};
use fdt_rs::{base::DevTree, prelude::{PropReader, FallibleIterator}};

#[no_mangle]
pub extern "C" fn kmain(heart_id: u32, device_tree: *const u8) -> ! {
    basic_allocator::init();

    sbi::init_io(&BASE).unwrap();    

    println!("Hello, world");
    println!("heart: {}", heart_id);
    println!("device tree: {:?}", device_tree);


    let sstatus = riscv::register::sstatus::read();
    println!("Sstatus {{");
    println!("  uie: {}", sstatus.uie());
    println!("  sie: {}", sstatus.sie());
    println!("  upie: {}", sstatus.upie());
    println!("  spie: {}", sstatus.spie());
    println!("  spp: {:?}", sstatus.spp());
    println!("  fs: {:?}", sstatus.fs());
    println!("  xs: {:?}", sstatus.xs());
    println!("  sum: {}", sstatus.sum());
    println!("  mxr: {}", sstatus.mxr());
    println!("  sd: {}", sstatus.sd());
    println!("}}");
   
    let sstatus = riscv::register::sie::read();
    println!("Sstatus {{");
    println!("  usoft: {}", sstatus.usoft());
    println!("  ssoft: {}", sstatus.ssoft());
    println!("  utimer: {}", sstatus.utimer());
    println!("  utimer: {}", sstatus.utimer());
    println!("  stimer: {}", sstatus.stimer());
    println!("  uext: {}", sstatus.uext());
    println!("  sext: {}", sstatus.sext()); 
    println!("}}");

    let stvec = riscv::register::stvec::read();
    println!("Stvec {{");
    println!("  address: {:x}", stvec.address());
    println!("  mode: {:?}", stvec.trap_mode());
    println!("}}");
   
    let stvec = trap_entry as *const u8;
    unsafe {
        riscv::register::stvec::write(stvec as usize, riscv::register::mtvec::TrapMode::Direct);
    }

    // let tree = unsafe { DevTree::from_raw_pointer(device_tree) }.expect("DevTree::from_raw_pointer");
    // print_tree(&mut *stdio().lock(), &tree).ok();
    println!();

    let b = unsafe {
        *(0x100 as *const u64)
    };
    
    println!("b = {:x}", b);

    #[cfg(test)]
    test_main();

    let shutdown = BASE.get_extension::<SystemShutdown>().unwrap().unwrap();
    shutdown.shutdown().expect("shudown");
    loop {}
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


#[naked]
#[no_mangle]
pub unsafe extern "C" fn trap_entry() {
    asm!(
        "addi  sp, sp, -31 * 8", /* Allocate stack space */
        "sd    x1,  0 * 8(sp)",  /* Push registers */
        "sd    x2,  1 * 8(sp)",
        "sd    x3,  2 * 8(sp)",
        "sd    x4,  3 * 8(sp)",
        "sd    x5,  4 * 8(sp)",
        "sd    x6,  5 * 8(sp)",
        "sd    x7,  6 * 8(sp)",
        "sd    x8,  7 * 8(sp)",
        "sd    x9,  8 * 8(sp)",
        "sd   x10,  9 * 8(sp)",
        "sd   x11, 10 * 8(sp)",
        "sd   x12, 11 * 8(sp)",
        "sd   x13, 12 * 8(sp)",
        "sd   x14, 13 * 8(sp)",
        "sd   x15, 14 * 8(sp)",
        "sd   x16, 15 * 8(sp)",
        "sd   x17, 16 * 8(sp)",
        "sd   x18, 17 * 8(sp)",
        "sd   x19, 18 * 8(sp)",
        "sd   x20, 19 * 8(sp)",
        "sd   x21, 20 * 8(sp)",
        "sd   x22, 21 * 8(sp)",
        "sd   x23, 22 * 8(sp)",
        "sd   x24, 23 * 8(sp)",
        "sd   x25, 24 * 8(sp)",
        "sd   x26, 25 * 8(sp)",
        "sd   x27, 26 * 8(sp)",
        "sd   x28, 27 * 8(sp)",
        "sd   x29, 28 * 8(sp)",
        "sd   x30, 29 * 8(sp)",
        "sd   x31, 30 * 8(sp)",
        "mv a0, sp",
        "call trap",
        /* Pop registers */
        "ld    x1,  0 * 8(sp)",
        "ld    x2,  1 * 8(sp)",
        "ld    x3,  2 * 8(sp)",
        "ld    x4,  3 * 8(sp)",
        "ld    x5,  4 * 8(sp)",
        "ld    x6,  5 * 8(sp)",
        "ld    x7,  6 * 8(sp)",
        "ld    x8,  7 * 8(sp)",
        "ld    x9,  8 * 8(sp)",
        "ld   x10,  9 * 8(sp)",
        "ld   x11, 10 * 8(sp)",
        "ld   x12, 11 * 8(sp)",
        "ld   x13, 12 * 8(sp)",
        "ld   x14, 13 * 8(sp)",
        "ld   x15, 14 * 8(sp)",
        "ld   x16, 15 * 8(sp)",
        "ld   x17, 16 * 8(sp)",
        "ld   x18, 17 * 8(sp)",
        "ld   x19, 18 * 8(sp)",
        "ld   x20, 19 * 8(sp)",
        "ld   x21, 20 * 8(sp)",
        "ld   x22, 21 * 8(sp)",
        "ld   x23, 22 * 8(sp)",
        "ld   x24, 23 * 8(sp)",
        "ld   x25, 24 * 8(sp)",
        "ld   x26, 25 * 8(sp)",
        "ld   x27, 26 * 8(sp)",
        "ld   x28, 27 * 8(sp)",
        "ld   x29, 28 * 8(sp)",
        "ld   x30, 29 * 8(sp)",
        "ld   x31, 30 * 8(sp)",

        "addi  sp, sp, 31 * 8", /* Deallocate stack space */

        "sret",
    );
    unreachable!()
}

#[no_mangle]
extern "C" fn trap(regs: &mut TrapRegisters) {
    let sepc = riscv::register::sepc::read();
    let sstatus = riscv::register::sstatus::read();
    let sip = riscv::register::sip::read();
    let sie = riscv::register::sie::read();
    let scause = riscv::register::scause::read();
    
    println!("In function trap");
    println!("sepc = {:}", sepc);
    println!("scause = 0x{:x}", scause.bits());
    println!("scause.code = {:?}", scause.code());
    println!("scause.cause = {:?}", scause.cause());
    println!("regs = {:#?}", regs);
    let instruction = unsafe {
        *(sepc as *const u32)
    };
    println!("pc = {:x}", sepc);
    println!("ins = {:x}", instruction);

    match scause.cause() {
        Trap::Interrupt(int) => println!("Interrupt: {:?}", int),
        Trap::Exception(ex) => panic!("Supervisor exception {:?}", ex),
    }    
}

static INDENT_STR: &'static str = "                                                                                                                                ";

fn indent(n: usize) -> &'static str {
    INDENT_STR.split_at(n).0
}

fn print_tree<W>(w: &mut W, tree: &DevTree<'_>) -> core::fmt::Result
    where W: Write+Sized 
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
                                    writeln!(ind, "{}: {:?} ({})", prop_name, prop_str, prop_str.len())?; 
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


mod panic {
    use core::panic::PanicInfo;
    use core::fmt::Write;

    use crate::io;
    use crate::sbi::stdio;
        
    #[panic_handler]
    #[no_mangle]
    pub fn panic(info: &PanicInfo) -> ! {
        let io = stdio();
        unsafe {
            io.force_unlock();
        }
        
        writeln!(&mut *io.lock() , "{info}");        
        abort();
    }

    #[no_mangle]
    pub extern "C" fn abort() -> ! {
        loop {
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
    // qemu::exit_qemu(qemu::ExitCode::Success);
}

#[test_case]
fn hello_world() {
    println!("Hello world!");
}