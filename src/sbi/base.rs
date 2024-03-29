use core::{error::Error, fmt::Display, mem::transmute};

use riscv::register::{marchid::Marchid, mimpid::Mimpid, mvendorid::Mvendorid};

use super::{
    call::{sbi_call0, sbi_call1},
    ExtensionId, FunctionId, SbiError, SbiResult,
};

static BASE_EXTENSION: SbiBaseExtension = SbiBaseExtension { _n: () };

pub fn base_extension() -> &'static SbiBaseExtension {
    &BASE_EXTENSION
}

pub trait SbiExtension {
    fn id() -> ExtensionId;
    unsafe fn from_probe(probe_result: isize) -> Self;
}

pub struct SbiBaseExtension {
    _n: (),
}

pub const BASE_GET_SPEC_VERSION: FunctionId = FunctionId(0x0);
pub const BASE_GET_IMP_ID: FunctionId = FunctionId(0x1);
pub const BASE_GET_IMP_VERSION: FunctionId = FunctionId(0x2);
pub const BASE_PROBE_EXT: FunctionId = FunctionId(0x3);
pub const BASE_GET_MVENDORID: FunctionId = FunctionId(0x4);
pub const BASE_GET_MARCHID: FunctionId = FunctionId(0x5);
pub const BASE_GET_MIMPID: FunctionId = FunctionId(0x6);

impl SbiExtension for SbiBaseExtension {
    fn id() -> ExtensionId {
        ExtensionId(0x10)
    }

    /// Should only be called with value returned from `sbi_probe_extension`
    unsafe fn from_probe(_i: isize) -> Self {
        SbiBaseExtension { _n: () }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SbiSpecVersion {
    pub major: u8,
    pub minor: u32,
}

impl From<isize> for SbiSpecVersion {
    fn from(i: isize) -> Self {
        let minor = (i | ((1 << 24) - 1)) as u32;
        let major = (i >> 24) as u8;
        Self { major, minor }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SbiImplementionId {
    BerkelyBootLoader,
    OpenSBI,
    Xvisor,
    KVM,
    RustSBI,
    Diosix,
    Coffer,
    Other(isize),
}

impl From<isize> for SbiImplementionId {
    fn from(i: isize) -> Self {
        use SbiImplementionId::*;
        match i {
            0 => BerkelyBootLoader,
            1 => OpenSBI,
            2 => Xvisor,
            3 => KVM,
            4 => RustSBI,
            5 => Diosix,
            6 => Coffer,
            _ => Other(i),
        }
    }
}

impl SbiBaseExtension {
    pub fn get_spec_version(&self) -> SbiResult<SbiSpecVersion> {
        unsafe { sbi_call0(Self::id(), BASE_GET_SPEC_VERSION).map(|i| SbiSpecVersion::from(i)) }
    }

    pub fn get_impl_id(&self) -> SbiResult<SbiImplementionId> {
        unsafe { sbi_call0(Self::id(), BASE_GET_IMP_ID).map(|i| SbiImplementionId::from(i)) }
    }

    pub fn get_impl_version(&self) -> SbiResult<isize> {
        unsafe { sbi_call0(Self::id(), BASE_GET_IMP_VERSION) }
    }

    pub fn get_extension<E>(&self) -> Result<E, GetExtensionError>
    where
        E: SbiExtension,
    {
        let id = E::id();
        let result = unsafe { sbi_call1(id.0 as usize, SbiBaseExtension::id(), BASE_PROBE_EXT) };
        match result {
            Ok(0) => Err(GetExtensionError::MissingExtension(id)),
            Ok(n) => unsafe { Ok(E::from_probe(n)) },
            Err(err) => Err(GetExtensionError::SbiError(err)),
        }
    }

    pub fn get_mvendorid(&self) -> SbiResult<Option<Mvendorid>> {
        unsafe { sbi_call0(Self::id(), BASE_GET_MVENDORID) }.map(|result| match result {
            0 => None,
            // Mvendorid only has a private constructor.
            n => Some(unsafe { transmute::<_, Mvendorid>(n) }),
        })
    }

    pub fn get_marchid(&self) -> SbiResult<Option<Marchid>> {
        unsafe { sbi_call0(Self::id(), BASE_GET_MARCHID) }.map(|result| match result {
            0 => None,
            // Mvendorid only has a private constructor.
            n => Some(unsafe { transmute::<_, Marchid>(n) }),
        })
    }

    pub fn get_mimpid(&self) -> SbiResult<Option<Mimpid>> {
        let result = unsafe { sbi_call0(Self::id(), BASE_GET_MIMPID)? };
        match result {
            0 => Ok(None),
            // Mvendorid only has a private constructor.
            n => Ok(Some(unsafe { transmute::<_, Mimpid>(n) })),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GetExtensionError {
    SbiError(SbiError),
    MissingExtension(ExtensionId),
}

impl Display for GetExtensionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GetExtensionError::SbiError(err) => err.fmt(f),
            GetExtensionError::MissingExtension(ext) => writeln!(f, "missing extension {}", ext),
        }
    }
}

impl Error for GetExtensionError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            GetExtensionError::SbiError(ref err) => Some(err),
            _ => None,
        }
    }
}
