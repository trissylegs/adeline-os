use core::arch::asm;

pub mod plic;

bitflags::bitflags! {
    pub struct Sip : usize {
        const SSIP = 1 << 1;
        const STIP = 1 << 5;
        const SEIP = 1 << 9;
    }
}

impl Sip {
    pub fn read() -> Sip {
        let mut bits;
        unsafe {
            asm!(
                "csrr {bits}, sip",
                bits = out(reg) bits
            );
        }
        Sip { bits }
    }

    pub fn write(sip: Sip) {
        unsafe {
            asm!(
                "csrw sip, {bits}",
                bits = in(reg) sip.bits
            );
        }
    }
}
