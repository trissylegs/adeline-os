
use core::fmt::{Debug, Write};

use riscv::register::{
    scause::{self, Trap},
    sepc, sie, sstatus, stval,
};

use crate::console::{LockOrDummy, self};
use crate::isr::Sip;

/// Registers saved to stack on
#[repr(C)]
pub struct TrapRegisters {
    /// Informative. Won't be restored on trap return. Use sepc
    pub pc: u64,
    pub ra: u64,
    /// Informative. Won't be restored on trap return.
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
            .field("pc", &format_args!("0x{:>08x}", &self.pc))
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

#[no_mangle]
#[allow(unused_must_use)]
extern "C" fn trap(registers: &mut TrapRegisters) {
    let sepc = sepc::read();
    let sstatus = sstatus::read();
    let sie_val = sie::read();
    let sip = Sip::read();
    let scause = scause::read();
    let stval = stval::read();

    let mut w = LockOrDummy::Dummy;

    writeln!(w, "sepc: {:?}", sepc);
    writeln!(w, "sstatus: {:?}", sstatus);
    writeln!(w, "sie: {:?}", sie_val);
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
                crate::time::interrupt_handler(w, registers);
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
            writeln!(console, "sie     = {:?}", sie_val).ok();
            writeln!(console, " .ssoft   = {:?}", sie_val.ssoft());
            writeln!(console, " .stimer  = {:?}", sie_val.stimer());
            writeln!(console, " .sext    = {:?}", sie_val.sext());
            writeln!(console, " .usoft   = {:?}", sie_val.usoft());
            writeln!(console, " .utimer  = {:?}", sie_val.utimer());
            writeln!(console, " .uext    = {:?}", sie_val.uext());
            writeln!(console, "scause  = 0x{:x}", scause.bits()).ok();
            writeln!(console, " .code  = {:?}", scause.code()).ok();
            writeln!(console, " .cause = {:?}", scause.cause()).ok();
            writeln!(console, "stval   = 0x{:x}", stval).ok();
            writeln!(console, "registers:").ok();
            writeln!(console, "  pc    = 0x{:x}", registers.pc);
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
