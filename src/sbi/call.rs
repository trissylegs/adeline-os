use core::arch::asm;
use super::*;



pub unsafe fn sbi_call0(ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call1(a0: usize, ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call2(a0: usize, a1: usize, ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call3(a0: usize, a1: usize, a2: usize, ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call4(a0: usize, a1: usize, a2: usize, a3: usize, ext: ExtensionId) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call5(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    ext: ExtensionId,
) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0,
        in("a1") a1,
        in("a2") a2,
        in("a3") a3,
        in("a4") a4,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call6(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    ext: ExtensionId,
) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
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

    return SbiRet {
        error: error.into(),
        value,
    };
}

pub unsafe fn sbi_call7(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    ext: ExtensionId,
) -> SbiRet {
    let mut error: isize;
    let mut value: isize;

    asm!(
        "ecall",
        in("a7") ext.0,
        in("a0") a0, 
        in("a1") a1, 
        in("a2") a2, 
        in("a3") a3, 
        in("a4") a4, 
        in("a5") a5, 
        in("a6") a6,
        lateout("a0") error,
        lateout("a1") value,
    );

    return SbiRet {
        error: error.into(),
        value,
    };
}
