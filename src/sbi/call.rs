use super::*;
use core::arch::asm;

pub unsafe fn sbi_call0(ext: ExtensionId, func: FunctionId) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call1(a0: usize, ext: ExtensionId, func: FunctionId) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call2(
    a0: usize,
    a1: usize,
    ext: ExtensionId,
    func: FunctionId,
) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call3(
    a0: usize,
    a1: usize,
    a2: usize,
    ext: ExtensionId,
    func: FunctionId,
) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call4(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    ext: ExtensionId,
    func: FunctionId,
) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call5(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    ext: ExtensionId,
    func: FunctionId,
) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a4") a4,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}

pub unsafe fn sbi_call6(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    ext: ExtensionId,
    func: FunctionId,
) -> SbiResult<isize> {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a6") func.0,
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a4") a4,
        in("a5") a5,
        lateout("a0") error,
        lateout("a1") value,
    );

    SbiRet {
        error: error.into(),
        value,
    }
    .into_result(ext, func)
}
