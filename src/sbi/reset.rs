use super::{call::sbi_call2, ExtensionId, FunctionId, SbiExtension, SbiResult};

pub struct SystemResetExtension {
    _probe_result: isize,
}

impl SbiExtension for SystemResetExtension {
    fn id() -> ExtensionId {
        // "SRST"
        ExtensionId(0x53525354)
    }

    unsafe fn from_probe(probe_result: isize) -> Self {
        SystemResetExtension {
            _probe_result: probe_result,
        }
    }
}

const SRST_RESET: FunctionId = FunctionId(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum ResetType {
    Shutdown = 0x00000000,
    ColdReboot = 0x00000001,
    WarmReboot = 0x00000002,
}

impl Into<usize> for ResetType {
    fn into(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum ResetReason {
    NoReason = 0x00000000,
    SystemFailure = 0x00000001,
}

impl Into<usize> for ResetReason {
    fn into(self) -> usize {
        self as usize
    }
}

impl SystemResetExtension {
    pub fn reset(&self, reset_type: ResetType, reason: ResetReason) -> SbiResult<!> {
        let result = unsafe { sbi_call2(reset_type.into(), reason.into(), Self::id(), SRST_RESET) };
        result.map(|v| panic!("Returned for System reset with success! value = {:?}", v))
    }
}
