use crate::console::sbi_console;

use core::fmt::Write;
use core::panic::PanicInfo;

#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let mut io = unsafe { sbi_console() };

    writeln!(io, "{info}").ok();
    abort();
}

#[cfg(not(features = "ndebug"))]
#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(features = "ndebug")]
#[no_mangle]
extern "C" fn abort() -> ! {
    use crate::sbi::reset::{ResetReason, ResetType, SYSTEM_RESET_EXTENSION};
    if let Some(srst) = SYSTEM_RESET_EXTENSION.get() {
        srst.reset(ResetType::Shutdown, ResetReason::SystemFailure)
            .ok();
    }

    #[allow(deprecated)]
    crate::sbi::_legacy_shutdown().ok();

    loop {}
}
