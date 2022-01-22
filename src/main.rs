
#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write, sync::atomic::{AtomicBool, Ordering}};
use fdt_rs::base::DevTree;

#[repr(transparent)]
struct ExtensionId(isize);

#[derive(Clone, Copy)]
struct SbiRet {
    error: SbiError,
    value: isize,
}

impl SbiRet {
    fn into_result(self) -> SbiResult<isize> {
        self.into()
    }
}

impl Into<SbiResult<isize>> for SbiRet {
    fn into(self) -> SbiResult<isize> {
        match self.error {
            SbiError::SbiSuccess => Ok(self.value),
            _ => Err(self.error)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SbiError {
    SbiSuccess             ,
    SbiErrFailed           ,
    SbiErrNotSupported     ,
    SbiErrInvalidParam     ,
    SbiErrDenied           ,
    SbiErrInvalidAddress   ,
    SbiErrAlreadyAvailable ,
    SbiErrAlreadyStarted   ,
    SbiErrAlreadyStopped   ,
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
            -4 => SbiErrDenied ,
            -5 => SbiErrInvalidAddress,
            -6 => SbiErrAlreadyAvailable,
            -7 => SbiErrAlreadyStarted,
            -8 => SbiErrAlreadyStopped,
            _ => Unknown(i)
        }
    }
}

impl Default for SbiError {
    fn default() -> Self {
        Self::SbiSuccess
    }
}

type SbiResult<T> = Result<T, SbiError>;

unsafe fn sbi_call0(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a4") a4,
        in("a5") a5,
        in("a6") a6,
        
        lateout("a0") error,
        lateout("a1") value,
    );

    

    return SbiRet { error: error.into(), value }
}


fn sbi_get_spec_version() -> SbiResult<isize> {
    unsafe {
        let result = sbi_call0(0, 0, 0, 0, 0, 0, 0, SbiBaseExtension::id());
        result.into()        
    }
}

fn sbi_probe_extension(extension_id: ExtensionId) -> SbiResult<isize> {
    unsafe {
        let result = sbi_call0(extension_id.0 as usize, 0, 0, 0, 0, 0, 0, SbiBaseExtension::id());
        result.into()
    }
}

trait SbiExtension {
    fn id() -> ExtensionId;
    unsafe fn from_probe(i: isize) -> Self;
}

struct SbiBaseExtension {
    _n: ()
}

const BASE: SbiBaseExtension = SbiBaseExtension { _n: () };

impl SbiExtension for SbiBaseExtension {
    fn id() -> ExtensionId {
        ExtensionId(0x10)
    }
    
    /// Should only be called with value returned from `sbi_probe_extension`
    unsafe fn from_probe(_i: isize) -> Self { SbiBaseExtension { _n: () } }
}

impl SbiBaseExtension {
    fn get_spec_version(&self) -> SbiResult<isize> {
        sbi_get_spec_version()
    }

    fn get_extension<E>(&self) -> SbiResult<Option<E>> where E: SbiExtension {
        match sbi_probe_extension(E::id())? {
            0 => Ok(None),
            n => unsafe {
                Ok(Some(E::from_probe(n)))
            }
        }
    }
}


struct ConsolePutChar { _n: ()}

impl SbiExtension for ConsolePutChar {
    fn id() -> ExtensionId {
        ExtensionId(0x01)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        Self { _n: () }
    }
}

impl ConsolePutChar {
    fn put_char(&self, ch: u8) {
        unsafe {
            sbi_call0(ch as usize, 0, 0, 0, 0, 0, 0, Self::id())
                .into_result()
                .expect("sbi_put_char");
        }
    }
}


struct  SystemShutdown {
    _n: ()
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
    fn shutdown(&self) -> Result<(), SbiError> {
        unsafe {
            let SbiRet { error, .. } = sbi_call0(0, 0, 0, 0, 0, 0, 0, Self::id());
            Err(error.into())
        }
    }
}

struct SbiIO {
    put_char: ConsolePutChar,
}

impl Write for SbiIO {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.put_char.put_char(b);
        }
        Ok(())
    }
}


static IO_INIT: AtomicBool = AtomicBool::new(false);

static mut IO: SbiIO = SbiIO {
    put_char: ConsolePutChar { _n: () }
};

fn io() -> Option<&'static mut SbiIO> {
    if IO_INIT.load(Ordering::SeqCst) {
        unsafe {
            Some(&mut IO)
        }
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn kmain(heart_id: u32, device_tree: *const u8) -> ! {

    let version = BASE.get_spec_version();

    match version {
        Ok(_value) => {
            /*  */
            let put_char = BASE.get_extension::<ConsolePutChar>().unwrap().unwrap();
            IO_INIT.store(true, Ordering::SeqCst);

            let mut writer = SbiIO { put_char: put_char };

            writeln!(writer, "Hello, world");
            writeln!(writer, "heart: {heart_id}");
            writeln!(writer, "device tree: {device_tree:?}");

            let tree = unsafe { DevTree::from_raw_pointer(device_tree) }
                .expect("DevTree::from_raw_pointer");

            print_tree(&mut writer, &tree);

            let shutdown = BASE.get_extension::<SystemShutdown>().unwrap().unwrap();
            shutdown.shutdown().expect("shudown");
            loop {}
        }
        Err(_error) => {
            /*  */
            loop {
                panic!("{_error:?}")
            }
        }
    }
}

fn print_tree<W>(w: &mut W, tree: &DevTree<'_>) where W: Write+Sized {
    let magic = tree.magic();
    let version = tree.version();
    let totalsize = tree.totalsize();

    let boot_cpuid_phys = tree.boot_cpuid_phys();
    let last_comp_version = tree.last_comp_version();
    let off_mem_rsvmap = tree.off_mem_rsvmap();
    let off_dt_struct = tree.off_dt_struct();
    let size_dt_struct = tree.size_dt_struct();
    let off_dt_strings = tree.off_dt_strings();
    let size_dt_strings = tree.off_dt_strings();


    writeln!(w, "DevTree:");

    let mut ind = indenter::indented(w);
    ind = ind.ind(4);
    writeln!(ind, "magic: {magic}");
    writeln!(ind, "version: {version}");
    writeln!(ind, "totalsize: {totalsize}");
    writeln!(ind, "boot_cpuid_phys: {boot_cpuid_phys}");
    writeln!(ind, "last_comp_version: {last_comp_version}");
    writeln!(ind, "off_mem_rsvmap: {off_mem_rsvmap}");
    writeln!(ind, "off_dt_struct: {off_dt_struct}");
    writeln!(ind, "size_dt_struct: {size_dt_struct}");
    writeln!(ind, "off_dt_strings: {off_dt_strings}");
    writeln!(ind, "size_dt_strings: {size_dt_strings}");

    writeln!(ind, "reserved_entries:");
    ind = ind.ind(8);
    for re in tree.reserved_entries() {
        
        let address: u64 = re.address.into();
        let size: u64 = re.size.into();
        writeln!(ind, "fdt_reserve_entry: ");
        writeln!(ind, "    address: {address:x}");
        writeln!(ind, "    size: {size:x}");        
    }
}

mod panic {
    use core::panic::PanicInfo;
    use core::fmt::Write;

    use crate::io;
        
    #[panic_handler]
    #[no_mangle]
    pub fn panic(info: &PanicInfo) -> ! {
        if let Some(io) = io() {
            writeln!(io, "{info}");
        }
        abort();
    }

    #[no_mangle]
    pub extern "C" fn abort() -> ! {
        loop {
        }
    }
}
