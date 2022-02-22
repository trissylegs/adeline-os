
#[allow(dead_code)]
mod call;
use call::*;
use conquer_once::spin::OnceCell;
use spin::Mutex;

#[repr(transparent)]
pub struct ExtensionId(isize);

#[derive(Clone, Copy)]
pub struct SbiRet {
    error: SbiError,
    value: isize,
}

impl SbiRet {
    pub fn into_result(self) -> SbiResult<isize> {
        self.into()
    }
}

impl Into<SbiResult<isize>> for SbiRet {
    fn into(self) -> SbiResult<isize> {
        match self.error {
            SbiError::SbiSuccess => Ok(self.value),
            _ => Err(self.error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SbiError {
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

impl From<isize> for SbiError {
    fn from(i: isize) -> Self {
        use SbiError::*;
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

impl Default for SbiError {
    fn default() -> Self {
        Self::SbiSuccess
    }
}

pub type SbiResult<T> = Result<T, SbiError>;

pub fn sbi_get_spec_version() -> SbiResult<isize> {
    unsafe {
        let result = sbi_call0(SbiBaseExtension::id());
        result.into()
    }
}

pub fn sbi_probe_extension(extension_id: ExtensionId) -> SbiResult<isize> {
    unsafe {
        let result = sbi_call1(
            extension_id.0 as usize,
            SbiBaseExtension::id(),
        );
        result.into()
    }
}

pub trait SbiExtension {
    fn id() -> ExtensionId;
    unsafe fn from_probe(i: isize) -> Self;
}

pub struct SbiBaseExtension {
    _n: (),
}

pub const BASE_EXTENSION: SbiBaseExtension = SbiBaseExtension { _n: () };

impl SbiExtension for SbiBaseExtension {
    fn id() -> ExtensionId {
        ExtensionId(0x10)
    }

    /// Should only be called with value returned from `sbi_probe_extension`
    unsafe fn from_probe(_i: isize) -> Self {
        SbiBaseExtension { _n: () }
    }
}

impl SbiBaseExtension {
    pub fn get_spec_version(&self) -> SbiResult<isize> {
        sbi_get_spec_version()
    }

    pub fn get_extension<E>(&self) -> SbiResult<Option<E>>
    where
        E: SbiExtension,
    {
        match sbi_probe_extension(E::id())? {
            0 => Ok(None),
            n => unsafe { Ok(Some(E::from_probe(n))) },
        }
    }
}

pub struct ConsolePutChar {
    _n: (),
}

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
            sbi_call1(ch as usize, Self::id())
                .into_result()
                .expect("sbi_put_char");
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

impl ConsoleGetChar {
    pub fn get_char(&self) -> SbiResult<Option<u8>> {
        unsafe {
            sbi_call0(Self::id())
                .into_result()
                .map(|i| if i >= 0 && i <= 255 { Some(i as u8) } else { None })
        }
    }
}

pub struct SystemShutdown {
    _n: (),
}

impl SbiExtension for SystemShutdown {
    fn id() -> ExtensionId {
        ExtensionId(0x08)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        SystemShutdown { _n: () }
    }
}

impl SystemShutdown {
    pub fn shutdown(&self) -> Result<!, SbiError> {
        unsafe {
            let SbiRet { error, .. } = sbi_call0(Self::id());
            Err(error.into())
        }
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
    let put_char = base.get_extension::<ConsolePutChar>()?        
        .unwrap();
    let get_char = base.get_extension::<ConsoleGetChar>()?
        .unwrap();
    _IO.init_once(|| {        
        Mutex::new(SbiIO { put_char, get_char })
    });
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