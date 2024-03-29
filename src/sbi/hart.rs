use core::fmt::Display;

use spin::Once;

use super::{
    base::SbiExtension,
    call::{sbi_call0, sbi_call1, sbi_call3},
    FunctionId, SbiResult,
};

pub static HSM_EXTENSION: Once<Hsm> = Once::INIT;

pub fn hsm_extension() -> &'static Hsm {
    HSM_EXTENSION.get().unwrap()
}

pub struct Hsm {
    _probe_result: isize,
}

const HSM_HART_START: FunctionId = FunctionId(0x0);
const HSM_HART_STOP: FunctionId = FunctionId(0x1);
const HSM_HART_GET_STATUS: FunctionId = FunctionId(0x2);
const HSM_HART_SUSPEND: FunctionId = FunctionId(0x3);

impl SbiExtension for Hsm {
    fn id() -> super::ExtensionId {
        // "HSM"
        super::ExtensionId(0x48534D)
    }

    unsafe fn from_probe(probe_result: isize) -> Self {
        Hsm {
            _probe_result: probe_result,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct HartId(pub usize);

impl From<u32> for HartId {
    fn from(n: u32) -> Self {
        HartId(n as usize)
    }
}
impl From<usize> for HartId {
    fn from(n: usize) -> Self {
        HartId(n)
    }
}
#[cfg(target_pointer_width = "64")]
impl From<u64> for HartId {
    fn from(n: u64) -> Self {
        HartId(n as usize)
    }
}

impl Display for HartId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hart #{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct HartMask {
    pub hart_mask: usize,
    pub hart_mask_base: usize,
}

impl HartMask {
    pub const fn new() -> HartMask {
        Self::with_base(HartId(0))
    }

    pub const fn with_base(base_id: HartId) -> HartMask {
        HartMask {
            hart_mask: 0,
            hart_mask_base: base_id.0,
        }
    }

    pub fn set_id(&mut self, id: HartId) {
        if self.hart_mask_base + id.0 >= (usize::BITS as usize) {
            panic!(
                "Hart ID #{} will not fit in mask with base: {}",
                id.0, self.hart_mask_base
            );
        }
        self.hart_mask_base |= 1 << (id.0 - self.hart_mask_base);
    }

    pub fn clear_id(&mut self, id: HartId) {
        if self.hart_mask_base + id.0 >= (usize::BITS as usize) {
            panic!(
                "Hart ID #{} will not fit in mask with base: {}",
                id.0, self.hart_mask_base
            );
        }
        self.hart_mask_base &= !(1 << (id.0 - self.hart_mask_base));
    }
}

impl From<core::ops::Range<usize>> for HartMask {
    fn from(range: core::ops::Range<usize>) -> Self {
        if range.len() >= (usize::BITS as usize) {
            panic!("Too many hart id's for mask")
        }
        let hart_mask_base = range.start;
        if range.len() == 0 {
            Self {
                hart_mask_base,
                hart_mask: 0,
            }
        } else {
            Self {
                hart_mask_base,
                hart_mask: ((1 << range.len()) - 1),
            }
        }
    }
}

impl IntoIterator for HartMask {
    type Item = HartId;

    type IntoIter = HartMarkIter;

    fn into_iter(self) -> Self::IntoIter {
        HartMarkIter { mask: self, bit: 0 }
    }
}

pub struct HartMarkIter {
    mask: HartMask,
    bit: u32,
}

impl Iterator for HartMarkIter {
    type Item = HartId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.bit >= usize::BITS {
                return None;
            }
            let next_bit = self.bit;
            self.bit += 1;
            if self.mask.hart_mask & (1 << next_bit) != 0 {
                return Some(HartId(self.mask.hart_mask_base + next_bit as usize));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some((usize::BITS - self.bit) as usize))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetentiveSuspendType(pub u32);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonRetentiveSuspendType(pub u32);

impl RetentiveSuspendType {
    pub const DEFAULT_RETENTIVE_SUSPEND: RetentiveSuspendType = RetentiveSuspendType(0x00000000);
}
impl Default for RetentiveSuspendType {
    fn default() -> Self {
        Self::DEFAULT_RETENTIVE_SUSPEND
    }
}

impl NonRetentiveSuspendType {
    pub const DEFAULT_NON_RETENTIVE_SUSPEND: NonRetentiveSuspendType =
        NonRetentiveSuspendType(0x80000000);
}

impl Default for NonRetentiveSuspendType {
    fn default() -> Self {
        Self::DEFAULT_NON_RETENTIVE_SUSPEND
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HartState {
    Started = 0,
    Stopped = 1,
    StartPending = 2,
    StopPending = 3,
    Suspended = 4,
    SuspendPending = 5,
    ResumePending = 6,
}

impl Hsm {
    pub unsafe fn hart_start(
        &self,
        hartid: HartId,
        start_addr: usize,
        opaque: usize,
    ) -> SbiResult<()> {
        sbi_call3(hartid.0, start_addr, opaque, Self::id(), HSM_HART_START)?;
        Ok(())
    }

    pub unsafe fn hart_stop(&self) -> SbiResult<!> {
        sbi_call0(Self::id(), HSM_HART_START)?;
        panic!("sbi_hart_stop RETURNED WITHOUT ERROR");
    }

    pub fn hart_get_status(&self, hartid: HartId) -> SbiResult<HartState> {
        let i = unsafe { sbi_call1(hartid.0, Self::id(), HSM_HART_GET_STATUS) }?;
        Ok(match i {
            0 => HartState::Started,
            1 => HartState::Stopped,
            2 => HartState::StartPending,
            3 => HartState::StopPending,
            4 => HartState::Suspended,
            5 => HartState::SuspendPending,
            6 => HartState::ResumePending,
            _ => panic!("Unknown hart state: {}", i),
        })
    }

    pub fn hart_retentive_suspend(&self, suspend_type: RetentiveSuspendType) -> SbiResult<()> {
        unsafe { self.hart_suspend(suspend_type.0, 0, 0) }
    }

    unsafe fn hart_non_retentive_suspend(
        &self,
        suspend_type: NonRetentiveSuspendType,
        resume_addr: usize,
        opaque: usize,
    ) -> SbiResult<()> {
        self.hart_suspend(suspend_type.0, resume_addr, opaque)
    }

    unsafe fn hart_suspend(
        &self,
        suspend_type: u32,
        resume_addr: usize,
        opaque: usize,
    ) -> SbiResult<()> {
        sbi_call3(
            suspend_type as usize,
            resume_addr,
            opaque,
            Self::id(),
            HSM_HART_SUSPEND,
        )?;
        Ok(())
    }
}
