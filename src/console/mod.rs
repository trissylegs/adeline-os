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
pub fn _print(args: core::fmt::Arguments, file: &str, line: u32, column: u32) {
    if let Some(uart) = NS16550A.get() {
        let mut lock = uart.lock();
        core::fmt::Write::write_fmt(&mut *lock, args).ok();
    } else {
        panic!("Attemmpted to print before console was initalized. {file}:{line}:{column}\n{args}")
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::_print(format_args!($($arg)*), file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::console::_print(format_args!("\n"), file!(), line!(), column!()) };
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

pub enum LockOrDummy {
    Dummy,
    Normal(MutexGuard<'static, MmioSerialPort>),
}

impl fmt::Write for LockOrDummy {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self {
            LockOrDummy::Dummy => Ok(()),
            LockOrDummy::Normal(n) => n.write_str(s),
        }
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        match self {
            LockOrDummy::Dummy => Ok(()),
            LockOrDummy::Normal(n) => n.write_char(c),
        }
    }

    fn write_fmt(self: &mut Self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        match self {
            LockOrDummy::Dummy => Ok(()),
            LockOrDummy::Normal(n) => n.write_fmt(args),
        }
    }
}

/// Get a writer if it's avalible. Otherwise get a dummy writer which does
pub(crate) fn lock_or_dummy() -> impl fmt::Write {
    match NS16550A.get().unwrap().try_lock() {
        Some(l) => LockOrDummy::Normal(l),
        None => LockOrDummy::Dummy,
    }
}

#[derive(Debug)]
enum PanicWriter {
    Fallback,
    Normal(MutexGuard<'static, MmioSerialPort>),
}

impl PanicWriter {
    fn fallback_write(&self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            #[allow(deprecated)]
            crate::sbi::_legacy_putchar(b);
        }
        Ok(())
    }
}

impl Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self {
            PanicWriter::Normal(w) => w.write_str(s),
            PanicWriter::Fallback => self.fallback_write(s),
        }
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        match self {
            PanicWriter::Normal(w) => w.write_char(c),
            PanicWriter::Fallback => self.fallback_write(&c.encode_utf8(&mut [0; 4])),
        }
    }

    fn write_fmt(mut self: &mut Self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        match self {
            PanicWriter::Fallback => core::fmt::write(&mut self, args),
            PanicWriter::Normal(w) => w.write_fmt(args),
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
