use crate::sbi::base::BASE_EXTENSION;
use crate::sbi::reset::{ResetReason, ResetType, SystemResetExtension};
use crate::sbi::stdio;
use core::fmt::Write;
use core::panic::PanicInfo;

#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let io = stdio();
    unsafe {
        io.force_unlock();
    }

    writeln!(&mut *io.lock(), "{info}").ok();
    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    if let Ok(Some(srst)) = BASE_EXTENSION.get_extension::<SystemResetExtension>() {
        srst.reset(ResetType::Shutdown, ResetReason::SystemFailure)
            .ok();
    }
    loop {}
}
