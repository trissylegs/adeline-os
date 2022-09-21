mod uart_ns16550a;

use core::fmt::{self, Write};
use core::str;
use spin::{Mutex, MutexGuard, Once};

use crate::console::uart_ns16550a::MmioSerialPort;
use crate::hwinfo::HwInfo;

static NS16550A: Once<Mutex<MmioSerialPort>> = Once::INIT;

pub fn init(info: &HwInfo) {
    NS16550A.call_once(|| {
        let mut sp = unsafe { MmioSerialPort::new(info.uart.reg.base) };
        sp.init();
        writeln!(sp, "Serial Port initialized!").ok();

        Mutex::new(sp)
    });
}

struct ForceUnlockedWriter(MutexGuard<'static, MmioSerialPort>);

impl fmt::Write for ForceUnlockedWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s)
    }
}

pub unsafe fn force_unlock() -> impl core::fmt::Write {
    if let Some(uart) = NS16550A.get() {
        uart.force_unlock();
        let lock = uart.lock();
        return ForceUnlockedWriter(lock);
    }

    loop {
        // There's no console to write panic messages to to.
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    if let Some(uart) = NS16550A.get() {
        let mut lock = uart.lock();
        core::fmt::Write::write_fmt(&mut *lock, args).ok();
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::console::_print(format_args!("\n")) };
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

/*
struct MmioSerialPort {
    data: AtomicPtr<u8>,
    int_en: AtomicPtr<u8>,
    fifo_ctrl: AtomicPtr<u8>,
    line_ctrl: AtomicPtr<u8>,
    modem_ctrl: AtomicPtr<u8>,
    line_sts: AtomicPtr<u8>,
    info:
}

impl MmioSerialPort {
    pub unsafe fn new(info: &'static UartNS16550a) -> Self {

    }
} */
