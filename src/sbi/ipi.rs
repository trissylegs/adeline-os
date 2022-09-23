use spin::Once;

use super::{base::SbiExtension, call::sbi_call2, hart::HartMask, SbiResult};

pub static IPI_EXTENSION: Once<IpiExtension> = Once::INIT;

pub fn ipi_extension() -> &'static IpiExtension {
    IPI_EXTENSION.get().unwrap()
}

pub struct IpiExtension {
    _probe_result: isize,
}

impl SbiExtension for IpiExtension {
    fn id() -> super::ExtensionId {
        super::ExtensionId::IPI
    }

    unsafe fn from_probe(probe_result: isize) -> Self {
        IpiExtension {
            _probe_result: probe_result,
        }
    }
}

impl IpiExtension {
    pub fn send_ipi<H>(&self, h: H) -> SbiResult<()>
    where
        HartMask: From<H>,
    {
        let hart_mask = HartMask::from(h);
        unsafe {
            sbi_call2(
                hart_mask.hart_mask,
                hart_mask.hart_mask_base,
                Self::id(),
                super::FunctionId(0),
            )
            .and(Ok(()))
        }
    }
}
