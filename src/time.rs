use core::{
    fmt::Write,
    num::NonZeroU64,
    ops::{Add, AddAssign, Sub, SubAssign},
    sync::atomic::{AtomicU64, Ordering},
};

pub use core::time::*;

use riscv::register::{self, sstatus};
use spin::Once;

use crate::{
    prelude::*,
    sbi::{base::BASE_EXTENSION, hart::Hsm, timer::Timer},
    TrapRegisters,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

static MTIME_PER_SECOND: AtomicU64 = AtomicU64::new(0);

static TIMER_EXTENSION: Once<Timer> = Once::INIT;

pub(crate) fn init_time(hwinfo: &crate::hwinfo::HwInfo) {
    MTIME_PER_SECOND.store(hwinfo.timebase_freq, Ordering::Relaxed);
    TIMER_EXTENSION.call_once(|| {
        BASE_EXTENSION
            .get_extension::<Timer>()
            .expect("error probing for sbi timer extension")
            .expect("no sbi timer extension")
    });

    // Fail early if something is wrong
    let _time = Instant::now();
}

fn get_mtime_per_second() -> u64 {
    let hz = MTIME_PER_SECOND.load(Ordering::Relaxed);
    NonZeroU64::new(hz)
        .unwrap_or_else(|| panic!("{} has not been initialzed", module_path!()))
        .get()
}

// Haven't decided how I'm dealing with 32-bit
#[cfg(target_pointer_width = "64")]
fn get_mtime() -> u64 {
    register::time::read() as u64
}

fn convert_mtime_to_duration(mtime: u64) -> Duration {
    let mtime_per_second = get_mtime_per_second();
    let secs = mtime / mtime_per_second;
    let subsec_t = mtime % mtime_per_second;

    if mtime_per_second == NANOS_PER_SECOND {
        Duration::new(secs, subsec_t as u32)
    } else if mtime_per_second < NANOS_PER_SECOND {
        let nanos_per_t = NANOS_PER_SECOND / mtime_per_second;
        let subsec_nanos = subsec_t * nanos_per_t;
        assert!(subsec_nanos < (u32::MAX as u64));
        Duration::new(secs, subsec_nanos as u32)
    } else {
        todo!("when freq is greater than 1GHz")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    since_zero: Duration,
}

impl Instant {
    /// Depends on hardware. May just be boot time.
    pub fn time_started() -> Instant {
        Instant {
            since_zero: Duration::ZERO,
        }
    }

    pub fn from_mtime(time: u64) -> Self {
        Instant {
            since_zero: convert_mtime_to_duration(time),
        }
    }

    pub fn to_mtime(&self) -> Option<u64> {
        let secs = self.since_zero.as_secs();
        let subsec_nanos = self.since_zero.subsec_nanos() as u64;

        let mtime_per_second = MTIME_PER_SECOND.load(Ordering::Relaxed);

        let ticks = secs.checked_mul(mtime_per_second)?;

        if mtime_per_second == NANOS_PER_SECOND {
            Some(ticks + subsec_nanos)
        } else if mtime_per_second < NANOS_PER_SECOND {
            let nanos_per_t = NANOS_PER_SECOND / mtime_per_second;
            let subsec_t = subsec_nanos / nanos_per_t;
            Some(ticks + subsec_t)
        } else {
            todo!("when freq is greater than 1GHz")
        }
    }

    pub fn now() -> Instant {
        Instant {
            since_zero: convert_mtime_to_duration(get_mtime()),
        }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier)
            .expect("eariler is later than self")
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.since_zero.checked_sub(earlier.since_zero)
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.since_zero.saturating_sub(earlier.since_zero)
    }

    pub fn elapsed(&self) -> Duration {
        let now = Self::now();
        now.checked_duration_since(*self)
            .expect("clock is running backwards")
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        match self.since_zero.checked_add(duration) {
            Some(dur) => Some(Instant { since_zero: dur }),
            None => None,
        }
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        match self.since_zero.checked_sub(duration) {
            Some(dur) => Some(Instant { since_zero: dur }),
            None => None,
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, rhs: Duration) -> Instant {
        self.checked_add(rhs)
            .expect("overflow when adding instant and duration")
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs)
            .expect("underflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        self.saturating_duration_since(rhs)
    }
}

pub fn sleep(duration: Duration) {
    let start = Instant::now();
    let until = start + duration;

    let hsm = BASE_EXTENSION
        .get_extension::<Hsm>()
        .expect("error probing for sbi hsm extension")
        .expect("no sbi hsm extension");

    loop {
        set_timer(until).expect("failed to to set timer");
        hsm.hart_rentative_suspend(
            crate::sbi::hart::RentativeSuspendType::DEFAULT_RETENTIVE_SUSPEND,
        )
        .expect("failed to suspend");

        let now = Instant::now();
        println!("until = {:?}, now = {:?}", until, now);
        if until < now {
            return;
        }
    }
}

pub static LAST_SET_TIMER: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn set_timer(instant: Instant) -> Result<(), crate::sbi::SbiError> {
    let new_time = instant.to_mtime().expect("instant overflows mtime");
    let time = TIMER_EXTENSION.get().expect("no timer extension");

    unsafe {
        sstatus::clear_sie();
    }
    let old_timer = LAST_SET_TIMER.load(Ordering::SeqCst);
    let r;
    if old_timer > new_time {
        r = time.set_timer(new_time);
        if r.is_ok() {
            LAST_SET_TIMER.store(new_time, Ordering::SeqCst);
        }
    } else {
        r = Ok(())
    }
    unsafe {
        sstatus::set_sie();
    }
    r
}

pub(crate) fn interrupt_handler(mut w: impl Write, _regs: &mut TrapRegisters) {
    let time = get_mtime();
    let last_set = LAST_SET_TIMER.load(Ordering::SeqCst);
    let timer = TIMER_EXTENSION.get().expect("no timer extension");

    if last_set < time {
        let mtime_per_second = MTIME_PER_SECOND.load(Ordering::Relaxed);

        // This implies that eventually the kernel crashes onces mtime runs out.
        // From the hardware i'm using now that'll take: 58455 average Gregorian years
        let new_time = last_set
            .checked_add(mtime_per_second)
            .expect("mtime overflow");

        if let Ok(_) = timer.set_timer(new_time) {
            LAST_SET_TIMER.store(new_time, Ordering::SeqCst);
        }
    }

    writeln!(w, "TIMER: {:?}", time).ok();
}
