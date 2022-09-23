use super::base::SbiExtension;

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
