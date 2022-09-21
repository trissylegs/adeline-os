mod uart_ns16550a;

use core::fmt::Write;
use spin::{Mutex, MutexGuard, Once};

use crate::console::uart_ns16550a::MmioSerialPort;
use crate::sbi::SbiIO;
use crate::{hwinfo::HwInfo, sbi};

static NS16550A: Once<Mutex<MmioSerialPort>> = Once::INIT;

pub fn init(info: &HwInfo) {
    NS16550A.call_once(|| {
        let mut sp = unsafe { MmioSerialPort::new(info.uart.reg.base) };
        sp.init();
        writeln!(sp, "Serial Port initialized!").ok();

        crate::sbi::block_sbi_console();

        Mutex::new(sp)
    });
}

enum UnlockedWriter {
    Uart(MutexGuard<'static, MmioSerialPort>),
    Sbi(MutexGuard<'static, SbiIO>),
}

impl core::fmt::Write for UnlockedWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self {
            UnlockedWriter::Uart(uart) => uart.write_str(s),
            UnlockedWriter::Sbi(sbi) => sbi.write_str(s),
        }
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        match self {
            UnlockedWriter::Uart(uart) => uart.write_char(c),
            UnlockedWriter::Sbi(sbi) => sbi.write_char(c),
        }
    }

    fn write_fmt(self: &mut Self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        match self {
            UnlockedWriter::Uart(uart) => uart.write_fmt(args),
            UnlockedWriter::Sbi(sbi) => sbi.write_fmt(args),
        }
    }
}

pub unsafe fn force_unlock() -> impl core::fmt::Write {
    if let Some(uart) = NS16550A.get() {
        uart.force_unlock();
        let lock = uart.lock();
        UnlockedWriter::Uart(lock)
    } else {
        sbi::stdio().force_unlock();
        let lock = sbi::stdio().lock();
        UnlockedWriter::Sbi(lock)
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    if let Some(uart) = NS16550A.get() {
        let mut lock = uart.lock();
        core::fmt::Write::write_fmt(&mut *lock, args).ok();
    } else {
        let mut lock = sbi::stdio().lock();
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
