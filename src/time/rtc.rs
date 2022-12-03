use ::time::OffsetDateTime;
use fdt_rs::spec::Phandle;

use spin::Once;

use crate::{hwinfo::HwInfo, isr::plic::InterruptId};

const TIME_LOW: u64 = 0x00;
const TIME_HIGH: u64 = 0x04;
const ALARM_LOW: u64 = 0x08;
const ALARM_HIGH: u64 = 0x0c;
const IRQ_ENABLED: u64 = 0x10;
const CLEAR_ALARM: u64 = 0x14;
const ALARM_STATUS: u64 = 0x18;
const CLEAR_INTERRUPT: u64 = 0x1c;

pub static RTC: Once<Goldfish> = Once::INIT;

pub fn init(hwinfo: &'static HwInfo) {
    Goldfish::init(hwinfo);
}

pub struct Goldfish {
    base: u64,
    interrupt: InterruptId,
    interrupt_parent: Phandle,
}

impl Goldfish {
    pub fn init(hwinfo: &HwInfo) -> &'static Goldfish {
        RTC.call_once(|| Goldfish {
            base: hwinfo.rtc.reg.start,
            interrupt: hwinfo.rtc.interrupt,
            interrupt_parent: hwinfo.rtc.interrupt_parent,
        })
    }

    pub fn get() -> &'static Goldfish {
        RTC.get().expect("rtc not initialized")
    }

    pub fn read_time(&self) -> i64 {
        let time_lo;
        let time_hi;
        unsafe {
            time_lo = ((self.base + TIME_LOW) as *const u32).read_volatile() as u64;
            time_hi = ((self.base + TIME_HIGH) as *const u32).read_volatile() as u64;
        }
        let time = (time_hi << 32 | time_lo) as i64;
        time
    }
}

pub trait TimeValue: Sized {
    fn from_unix_nanos(i: i128) -> Self;

    fn now_utc() -> Self {
        let time = Goldfish::get().read_time();
        Self::from_unix_nanos(time as i128)
    }
}

impl TimeValue for OffsetDateTime {
    fn from_unix_nanos(i: i128) -> Self {
        OffsetDateTime::from_unix_timestamp_nanos(i).expect("unix timestamp overflowed")
    }
}
