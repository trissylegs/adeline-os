#[allow(dead_code)]
mod call;

use core::fmt::{self, Display, Formatter};

use call::*;
use conquer_once::spin::OnceCell;

use spin::Mutex;

use self::base::{SbiBaseExtension, SbiExtension, BASE_EXTENSION};

pub mod base;
pub mod hart;
pub mod reset;
pub mod timer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ExtensionId(isize);

impl ExtensionId {
    // Base extension. Used to look up other extensions.
    const BASE: ExtensionId = ExtensionId(0x10);
    // Legacy extensions. Prefer to use non-legacy alternatives when possible.
    const LEGACY_SET_TIMER: ExtensionId = ExtensionId(0x00);
    const LEGACY_CONSOLE_PUTCHAR: ExtensionId = ExtensionId(0x01);
    const LEGACY_CONSOLE_GETCHAR: ExtensionId = ExtensionId(0x02);
    const LEGACY_CLEAR_IPI: ExtensionId = ExtensionId(0x03);
    const LEGACY_SEND_IPI: ExtensionId = ExtensionId(0x04);
    const LEGACY_REMOTE_FENCE_I: ExtensionId = ExtensionId(0x05);
    const LEGACY_REMOTE_SFENCE_VMA: ExtensionId = ExtensionId(0x06);
    const LEGACY_REMOTE_SFENCE_VMA_WITH_ASID: ExtensionId = ExtensionId(0x07);
    const LEGACY_SYSTEM_SHUTDOWN: ExtensionId = ExtensionId(0x08);

    // Normal extensions.
    const TIMER: ExtensionId = ExtensionId(0x54494D45);
    const IPI: ExtensionId = ExtensionId(0x735049);
    const RFENCE: ExtensionId = ExtensionId(0x52464E43);
    const HSM: ExtensionId = ExtensionId(0x48534D);
    const SRST: ExtensionId = ExtensionId(0x53525354);
    const PMU: ExtensionId = ExtensionId(0x504D55);

    pub const fn is_legacy(self) -> bool {
        self.0 >= Self::LEGACY_SET_TIMER.0 && self.0 <= Self::LEGACY_SYSTEM_SHUTDOWN.0
    }

    pub const fn desc(self) -> Option<&'static str> {
        Some(match self {
            Self::BASE => "Base Extension",
            Self::LEGACY_SET_TIMER => "Legacy Set Timer",
            Self::LEGACY_CONSOLE_PUTCHAR => "Legacy Console Putchar",
            Self::LEGACY_CONSOLE_GETCHAR => "Legacy Console Getchar",
            Self::LEGACY_CLEAR_IPI => "Legacy Clear IPI",
            Self::LEGACY_SEND_IPI => "Legacy Send IPI",
            Self::LEGACY_REMOTE_FENCE_I => "Legacy Remote FENCE.I",
            Self::LEGACY_REMOTE_SFENCE_VMA => "Legacy Remote SFENCE.VMA",
            Self::LEGACY_REMOTE_SFENCE_VMA_WITH_ASID => "Legacy Remote SFENCE.VMA with ASID",
            Self::LEGACY_SYSTEM_SHUTDOWN => "Legacy System Shutdown",
            Self::TIMER => "Timer Extension",
            Self::IPI => "IPI Extension",
            Self::RFENCE => "Hart State Management Extension",
            Self::SRST => "System Reset Extension",
            Self::PMU => "Performance Moniotoring Unit Extension",
            _ if self.0 >= 0x08000000 && self.0 <= 0x08FFFFFF => "Experimental SBI Extension",
            _ if self.0 >= 0x09000000 && self.0 <= 0x09FFFFFF => "Vendor-Specific SBI Extension",
            _ if self.0 >= 0x0A000000 && self.0 <= 0x0AFFFFFF => "Firmware Specific SBI Extension",
            _ => return None,
        })
    }
}

impl Display for ExtensionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.desc() {
            Some(desc) => writeln!(f, "{} (EID #0x{:x})", desc, self.0),
            None => writeln!(f, "Unknown Extension (EID #0x{:x})", self.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FunctionId(isize);

impl FunctionId {
    fn desc(self, ext: ExtensionId) -> Option<&'static str> {
        match ext {
            ExtensionId::BASE => match self.0 {
                0 => Some("Get SBI specification version"),
                1 => Some("Get SBI implementaion ID"),
                2 => Some("Get SBI implementaion version"),
                3 => Some("Probe SBI extension"),
                4 => Some("Get machine vendor ID"),
                5 => Some("Get machine architecture ID"),
                6 => Some("Get machine implementation ID"),
                _ => None,
            },
            ExtensionId::TIMER => match self.0 {
                0 => Some("Set Timer"),
                _ => None,
            },
            ExtensionId::IPI => match self.0 {
                0 => Some("Send IPI"),
                _ => None,
            },
            ExtensionId::RFENCE => match self.0 {
                0 => Some("Remote FENCE.I"),
                1 => Some("Remote SFENCE.VMA"),
                2 => Some("Remote SFENCE.VMA with ASID"),
                3 => Some("Remote HFENCE.GVMA with VMID"),
                4 => Some("Remote HFENCE.GVMA"),
                5 => Some("Remote HFENCE.VVMA with ASID"),
                6 => Some("Remote HFENCE.VVMA"),
                _ => None,
            },
            ExtensionId::HSM => match self.0 {
                0 => Some("HART start"),
                1 => Some("HART stop"),
                2 => Some("HART get status"),
                3 => Some("HART suspend"),
                _ => None,
            },
            ExtensionId::SRST => match self.0 {
                0 => Some("System reset"),
                _ => None,
            },
            ExtensionId::PMU => match self.0 {
                0 => Some("Get number of counters"),
                1 => Some("Get details of a counter"),
                2 => Some("Find and configure a matching counter"),
                3 => Some("Start a set of counters"),
                4 => Some("Stop a set of counters"),
                5 => Some("Read a firmware counter"),
                _ => None,
            },
            _ => None,
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct SbiError {
    pub code: SbiErrorCode,
    pub extension: ExtensionId,
    pub function: FunctionId,
}

impl Display for SbiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SBI Error {:?} (EID #{}, FID #{})",
            self.code, self.extension.0, self.function.0
        )?;
        if let Some(extension) = self.extension.desc() {
            write!(f, ": {}", extension)?;
            if let Some(func_desc) = self.function.desc(self.extension) {
                write!(f, ", {}", func_desc);
            }
        }

        Ok(())
    }
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

pub fn init_io() -> SbiResult<()> {
    let put_char = BASE_EXTENSION.get_extension::<ConsolePutChar>()?.unwrap();
    let get_char = BASE_EXTENSION.get_extension::<ConsoleGetChar>()?.unwrap();
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
