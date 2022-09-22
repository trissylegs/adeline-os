mod uart_ns16550a;

use core::fmt::{self, Write};
use core::str;
use spin::{Mutex, MutexGuard, Once};

use crate::console::uart_ns16550a::MmioSerialPort;
use crate::hwinfo::HwInfo;

static NS16550A: Once<Mutex<MmioSerialPort>> = Once::INIT;

pub fn init(info: &HwInfo) {
    NS16550A.call_once(|| {
        let uart = &info.uart;
        let mut sp = unsafe { MmioSerialPort::new(uart.reg.base, uart.interrupt) };
        sp.init().expect("failed to inialize serial port");
        writeln!(sp, "Serial Port initialized!").ok();

        Mutex::new(sp)
    });
}

pub(crate) fn enable_interrupts() {
    // NS16550A.get().unwrap().lock().enable_interrupts();
}

struct PendingBytes {
    uart: &'static Mutex<MmioSerialPort>,
}

impl Iterator for PendingBytes {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.uart.lock().try_receive()
    }
}

pub(crate) fn pending_bytes() -> impl Iterator<Item = u8> {
    let uart = NS16550A.get().expect("Serial Port initialized");
    PendingBytes { uart }
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

#[derive(Debug)]
struct LockHandle(MutexGuard<'static, MmioSerialPort>);

impl fmt::Write for LockHandle {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s)
    }
}

pub(crate) fn lock() -> impl fmt::Write {
    let lock = NS16550A.get().unwrap().lock();
    LockHandle(lock)
}

#[derive(Debug)]
enum PanicWriter {
    Fallback,
    Normal(MutexGuard<'static, MmioSerialPort>),
}

impl Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self {
            PanicWriter::Normal(w) => w.write_str(s),
            PanicWriter::Fallback => {
                for b in s.bytes() {
                    #[allow(deprecated)]
                    crate::sbi::_legacy_putchar(b);
                }
                Ok(())
            }
        }
    }
}

#[doc(hidden)]
pub(crate) unsafe fn _panic_unlock() -> impl fmt::Write {
    match NS16550A.get() {
        Some(lock) => {
            unsafe { lock.force_unlock() };
            PanicWriter::Normal(lock.lock())
        }
        None => PanicWriter::Fallback,
    }
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
