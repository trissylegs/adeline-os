use core::fmt::Write;
use spin::{Mutex, Once};
use uart_16550::MmioSerialPort;

use crate::hwinfo::HwInfo;

static NS16550A: Once<Mutex<MmioSerialPort>> = Once::INIT;

pub fn init(info: &HwInfo) {
    NS16550A.call_once(|| {
        let mut sp = unsafe { MmioSerialPort::new(info.uart.reg.base) };
        sp.init();
        writeln!(sp, "Serial Port initialized!").ok();
        writeln!(sp, "Press anything to continue.").ok();

        loop {
            let recv = sp.receive();
            writeln!(sp, "Recived {}", recv).ok();
        }

        Mutex::new(sp)
    });
}
