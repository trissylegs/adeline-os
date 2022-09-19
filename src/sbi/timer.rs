use super::{call::sbi_call1, FunctionId, SbiExtension, SbiResult};

pub struct Timer {
    _n: (),
}

const TIME_SET_TIMER: FunctionId = FunctionId(0);

impl SbiExtension for Timer {
    fn id() -> super::ExtensionId {
        // This is "TIME" in ascii.
        super::ExtensionId(0x54494D45)
    }

    unsafe fn from_probe(_i: isize) -> Self {
        Timer { _n: () }
    }
}

impl Timer {
    #[cfg(target_pointer_width = "64")]
    pub fn set_timer(&self, stime_value: u64) -> SbiResult<()> {
        unsafe {
            // We're on 64-bit so usize==u64
            sbi_call1(stime_value as usize, Self::id(), TIME_SET_TIMER)
                .into_result()
                .map(|_| ())
        }
    }

    #[cfg(target_pointer_width = "32")]
    fn set_timer(&self, stime_value: u64) -> SbiResult<()> {
        unsafe {
            let lo = stime_value as u32;
            let hi = (stime_value >> 32) as u32;

            sbi_call2(lo as usize, hi as usize, Self::id())
                .into_result()
                .map(|_| ())
        }
    }
}
