#[allow(dead_code)]
mod call;

use call::*;
use conquer_once::spin::OnceCell;

use spin::Mutex;

use self::base::{SbiBaseExtension, SbiExtension};

pub mod base;
pub mod hart;
pub mod reset;
pub mod timer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExtensionId(isize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FunctionId(isize);

#[derive(Clone, Copy)]
pub struct SbiRet {
    error: SbiErrorCode,
    value: isize,
}

impl SbiRet {
    pub fn into_result(self, extension: ExtensionId, function: FunctionId) -> SbiResult<isize> {
        let res: Result<isize, SbiErrorCode> = self.into();

        res.map_err(|code| SbiError {
            code,
            extension,
            function,
        })
    }
}

impl Into<Result<isize, SbiErrorCode>> for SbiRet {
    fn into(self) -> Result<isize, SbiErrorCode> {
        match self.error {
            SbiErrorCode::SbiSuccess => Ok(self.value),
            _ => Err(self.error),
        }
    }
}

#[derive(Debug)]
pub struct SbiError {
    pub code: SbiErrorCode,
    pub extension: ExtensionId,
    pub function: FunctionId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SbiErrorCode {
    #[default]
    SbiSuccess,
    SbiErrFailed,
    SbiErrNotSupported,
    SbiErrInvalidParam,
    SbiErrDenied,
    SbiErrInvalidAddress,
    SbiErrAlreadyAvailable,
    SbiErrAlreadyStarted,
    SbiErrAlreadyStopped,
    Unknown(isize),
}

impl From<isize> for SbiErrorCode {
    fn from(i: isize) -> Self {
        use SbiErrorCode::*;
        match i {
            0 => SbiSuccess,
            -1 => SbiErrFailed,
            -2 => SbiErrNotSupported,
            -3 => SbiErrInvalidParam,
            -4 => SbiErrDenied,
            -5 => SbiErrInvalidAddress,
            -6 => SbiErrAlreadyAvailable,
            -7 => SbiErrAlreadyStarted,
            -8 => SbiErrAlreadyStopped,
            _ => Unknown(i),
        }
    }
}

pub type SbiResult<T> = Result<T, SbiError>;

pub struct ConsolePutChar {
    _n: (),
}

const CONSOLE_PUTCHAR: FunctionId = FunctionId(0x0);

impl SbiExtension for ConsolePutChar {
    fn id() -> ExtensionId {
        ExtensionId(0x01)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        Self { _n: () }
    }
}

impl ConsolePutChar {
    pub fn put_char(&self, ch: u8) {
        unsafe {
            sbi_call1(ch as usize, Self::id(), CONSOLE_PUTCHAR).expect("sbi_put_char");
        }
    }
}

pub struct ConsoleGetChar {
    _n: (),
}

impl SbiExtension for ConsoleGetChar {
    fn id() -> ExtensionId {
        ExtensionId(0x02)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        Self { _n: () }
    }
}

const CONSOLE_GETCHAR: FunctionId = FunctionId(0);

impl ConsoleGetChar {
    pub fn get_char(&self) -> SbiResult<Option<u8>> {
        let i = unsafe { sbi_call0(Self::id(), CONSOLE_GETCHAR)? };
        if i >= 0 && i <= 255 {
            Ok(Some(i as u8))
        } else {
            Ok(None)
        }
    }
}

pub struct SystemShutdown {
    _n: (),
}

const SYSTEM_SHUTDOWN: FunctionId = FunctionId(0x0);

impl SbiExtension for SystemShutdown {
    fn id() -> ExtensionId {
        ExtensionId(0x08)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        SystemShutdown { _n: () }
    }
}

impl SystemShutdown {
    #[deprecated = "Use system reset extension"]
    pub fn shutdown(&self) -> Result<!, SbiError> {
        unsafe { sbi_call0(Self::id(), SYSTEM_SHUTDOWN)? };
        panic!("sbi_system_shutdown_returned")
    }
}

pub struct SbiIO {
    put_char: ConsolePutChar,
    get_char: ConsoleGetChar,
}

impl SbiIO {
    pub fn put_char(&mut self, ch: u8) {
        self.put_char.put_char(ch)
    }
}

impl core::fmt::Write for SbiIO {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.put_char.put_char(b);
        }
        Ok(())
    }
}

pub static _IO: OnceCell<Mutex<SbiIO>> = OnceCell::uninit();

impl SbiIO {
    pub fn get_char(&self) -> Option<u8> {
        self.get_char.get_char().ok().flatten()
    }
}

pub fn stdio() -> &'static Mutex<SbiIO> {
    _IO.get().unwrap()
}

pub fn init_io(base: &SbiBaseExtension) -> SbiResult<()> {
    let put_char = base.get_extension::<ConsolePutChar>()?.unwrap();
    let get_char = base.get_extension::<ConsoleGetChar>()?.unwrap();
    _IO.init_once(|| Mutex::new(SbiIO { put_char, get_char }));
    Ok(())
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    let mut lock = stdio().lock();
    core::fmt::Write::write_fmt(&mut *lock, args).ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::sbi::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::sbi::_print(format_args!("\n")) };
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}
