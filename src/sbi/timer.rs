use cfg_if::cfg_if;

use super::{
    call::{sbi_call1, sbi_call2},
    FunctionId, SbiExtension, SbiResult,
};

pub struct Timer {
    _probe_result: isize,
}

const TIMER_SET_TIMER: FunctionId = FunctionId(0);

impl SbiExtension for Timer {
    fn id() -> super::ExtensionId {
        // This is "TIME" in ascii.
        super::ExtensionId(0x54494D45)
    }

    unsafe fn from_probe(probe_result: isize) -> Self {
        Timer {
            _probe_result: probe_result,
        }
    }
}

impl Timer {
    pub fn set_timer(&self, stime_value: u64) -> SbiResult<()> {
        cfg_if! {
            if #[cfg(target_pointer_width = "32")] {
                self.set_timer_32(stime_value)
            }
            else if #[cfg(target_pointer_width = "64")] {
                self.set_timer_64(stime_value)
            }
            else {
                todo!("rv128")
            }
        }
    }

    // #[cfg(target_pointer_width = "32")]
    fn set_timer_32(&self, stime_value: u64) -> SbiResult<()> {
        unsafe {
            let lo = stime_value as u32;
            let hi = (stime_value >> 32) as u32;

            sbi_call2(lo as usize, hi as usize, Self::id(), TIMER_SET_TIMER)?;
            Ok(())
        }
    }

    fn set_timer_64(&self, stime_value: u64) -> SbiResult<()> {
        unsafe {
            // We're on 64-bit so usize==u64
            sbi_call1(stime_value as usize, Self::id(), TIMER_SET_TIMER)?;
            Ok(())
        }
    }
}
