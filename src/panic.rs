use crate::console;
use crate::sbi::base::BASE_EXTENSION;
use crate::sbi::reset::{ResetReason, ResetType, SystemResetExtension};
use core::fmt::Write;
use core::panic::PanicInfo;

#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let mut io = unsafe { console::_panic_unlock() };

    writeln!(io, "{info}").ok();
    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    if let Ok(Some(srst)) = BASE_EXTENSION.get_extension::<SystemResetExtension>() {
        srst.reset(ResetType::Shutdown, ResetReason::SystemFailure)
            .ok();
    }

    #[allow(deprecated)]
    crate::sbi::_legacy_shutdown().ok();

    loop {}
}
