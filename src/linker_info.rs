use core::fmt::Write;
use core::{ffi::c_void, ops::Range};

use spin::Once;

use crate::console;

extern "C" {
    pub static mut __image_start: u8;
    pub static mut __image_end: u8;
    pub static mut __text_start: u8;
    pub static mut __text_end: u8;
    pub static mut __rodata_start: u8;
    pub static mut __rodata_end: u8;
    pub static mut __data_start: u8;
    pub static mut __data_end: u8;
    pub static mut __bss_start: u8;
    pub static mut __bss_end: u8;
    pub static mut __stack_limit: u8;
    pub static mut __stack_top: u8;
    pub static mut __tdata_start: u8;
    pub static mut __tdata_end: u8;
    pub static mut __tbss_start: u8;
    pub static mut __tbss_end: u8;

    pub static mut __global_pointer: c_void;
}

unsafe fn range_from(start: &'static u8, end: &'static u8) -> Range<u64> {
    let ptr_start = start as *const _;
    let ptr_end = end as *const _;
    (ptr_start as u64)..(ptr_end as u64)
}

pub fn image() -> Range<u64> {
    unsafe { range_from(&__image_start, &__image_end) }
}

pub fn text() -> Range<u64> {
    unsafe { range_from(&__text_start, &__text_end) }
}

pub fn rodata() -> Range<u64> {
    unsafe { range_from(&__rodata_start, &__rodata_end) }
}

pub fn data() -> Range<u64> {
    unsafe { range_from(&__data_start, &__data_end) }
}

pub fn bss() -> Range<u64> {
    unsafe { range_from(&__bss_start, &__bss_end) }
}

pub fn tdata() -> Range<u64> {
    unsafe { range_from(&__tdata_start, &__tdata_end) }
}

pub fn tbss() -> Range<u64> {
    unsafe { range_from(&__tbss_start, &__tbss_end) }
}

macro_rules! write_address {
    ($w:ident, $var:ident) => {
        writeln!($w, "{:30}:   {:>16?}", stringify!($var), &$var as *const u8).ok();
    };
}

pub fn print_address_ranges() {
    let mut w = console::lock();
    writeln!(w, "image   0x{:x}..0x{:x}", image().start, image().end).ok();
    writeln!(w, "text    0x{:x}..0x{:x}", text().start, text().end).ok();
    writeln!(w, "rodata  0x{:x}..0x{:x}", rodata().start, rodata().end).ok();
    writeln!(w, "data    0x{:x}..0x{:x}", data().start, data().end).ok();
    writeln!(w, "bss     0x{:x}..0x{:x}", bss().start, bss().end).ok();
    writeln!(w, "tdata   0x{:x}..0x{:x}", tdata().start, tdata().end).ok();
    writeln!(w, "tbss    0x{:x}..0x{:x}", tbss().start, tbss().end).ok();
}

pub unsafe fn print_address() {
    let mut w = console::lock();
    write_address!(w, __image_start);
    write_address!(w, __image_end);
    write_address!(w, __text_start);
    write_address!(w, __text_end);
    write_address!(w, __rodata_start);
    write_address!(w, __rodata_end);
    write_address!(w, __data_start);
    write_address!(w, __data_end);
    write_address!(w, __bss_start);
    write_address!(w, __bss_end);
    write_address!(w, __stack_limit);
    write_address!(w, __stack_top);
    write_address!(w, __tdata_start);
    write_address!(w, __tdata_end);
    write_address!(w, __tbss_start);
    write_address!(w, __tbss_end);
}

/// LinkerInfo contains the address ranges of the kernel image. Set using linker script.
pub struct LinkerInfo {
    pub image: Range<u64>,
    pub text: Range<u64>,
    pub rodata: Range<u64>,
    pub data: Range<u64>,
    pub bss: Range<u64>,
    pub tdata: Range<u64>,
    pub tbss: Range<u64>,
}

static LINKER_INFO: Once<LinkerInfo> = Once::INIT;

impl LinkerInfo {
    pub fn get() -> &'static Self {
        LINKER_INFO.call_once(|| LinkerInfo {
            image: image(),
            text: text(),
            rodata: rodata(),
            data: data(),
            bss: bss(),
            tdata: tdata(),
            tbss: tbss(),
        })
    }
}
