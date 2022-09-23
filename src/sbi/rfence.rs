use spin::Once;

use super::base::SbiExtension;

pub static RFENCE_EXTENSION: Once<RfenceExtension> = Once::INIT;

pub fn rfence_extension() -> &'static RfenceExtension {
    RFENCE_EXTENSION.get().unwrap()
}

pub struct RfenceExtension {
    _probe_result: isize,
}

impl SbiExtension for RfenceExtension {
    fn id() -> super::ExtensionId {
        super::ExtensionId::RFENCE
    }

    unsafe fn from_probe(probe_result: isize) -> Self {
        RfenceExtension {
            _probe_result: probe_result,
        }
    }
}
