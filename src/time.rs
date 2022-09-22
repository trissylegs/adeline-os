use core::{
    iter::Sum,
    num::TryFromIntError,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration(u64);

pub const MILLIS_PER_SECOND: u64 = 1000;
pub const MICROS_PER_SECOND: u64 = 1_000_000;

pub const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub const NANOS_PER_MILLIS: u64 = NANOS_PER_SECOND / 1000;
pub const NANOS_PER_MICROS: u64 = 1000;

impl Duration {
    pub const ZERO: Duration = Self(0);
    pub const MAX: Duration = Self(u64::MAX);
    pub const SECOND: Duration = Self::from_secs(1);

    /// Construct from seconds and nanos. Here to line up with std::time::Duration.
    /// Even though internally we're storing nanoseconds as u64
    pub const fn new(secs: u64, nanos: u32) -> Self {
        Self::from_nanos((secs as u64) * NANOS_PER_SECOND + nanos as u64)
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn from_secs(secs: u64) -> Self {
        Self::from_nanos(secs * NANOS_PER_SECOND)
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self::from_nanos(millis * NANOS_PER_MILLIS)
    }

    pub const fn from_micros(micros: u64) -> Self {
        Self::from_nanos(micros * NANOS_PER_MICROS)
    }

    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub const fn as_secs(&self) -> u64 {
        self.0 / NANOS_PER_SECOND
    }

    pub const fn subsec_millis(&self) -> u32 {
        (self.subsec_nanos() / NANOS_PER_MILLIS as u32) as u32
    }

    pub const fn subsec_micros(&self) -> u32 {
        (self.subsec_nanos() / NANOS_PER_MICROS as u32) as u32
    }

    pub const fn subsec_nanos(&self) -> u32 {
        (self.0 % NANOS_PER_SECOND) as u32
    }

    pub const fn as_millis(&self) -> u64 {
        self.as_nanos() / (NANOS_PER_MILLIS as u64)
    }

    pub const fn as_micros(&self) -> u64 {
        self.as_nanos() / (NANOS_PER_MICROS as u64)
    }

    pub const fn as_nanos(&self) -> u64 {
        self.0
    }

    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        match self.0.checked_add(rhs.0) {
            Some(nanos) => Some(Duration(nanos)),
            None => None,
        }
    }

    pub const fn saturating_add(self, rhs: Duration) -> Duration {
        Duration(self.0.saturating_add(rhs.0))
    }

    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        match self.0.checked_sub(rhs.0) {
            Some(nanos) => Some(Duration(nanos)),
            None => None,
        }
    }

    pub const fn saturating_sub(self, rhs: Duration) -> Duration {
        Duration(self.0.saturating_sub(rhs.0))
    }

    pub const fn checked_mul(self, rhs: u32) -> Option<Duration> {
        match self.0.checked_mul(rhs as u64) {
            Some(nanos) => Some(Duration(nanos)),
            None => None,
        }
    }

    pub const fn saturating_mul(self, rhs: u32) -> Duration {
        Duration(self.0.saturating_mul(rhs as u64))
    }

    pub const fn checked_div(self, rhs: u32) -> Option<Duration> {
        match self.0.checked_div(rhs as u64) {
            Some(nanos) => Some(Duration(nanos)),
            None => None,
        }
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs)
            .expect("overflow when adding durations")
    }
}

impl AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs)
            .expect("underflow when subtracting durations")
    }
}

impl SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs
    }
}

impl Mul<Duration> for u32 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        Duration(
            (self as u64)
                .checked_mul(rhs.0)
                .expect("overflow when multiplying scalar by duration"),
        )
    }
}

impl Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u32) -> Self::Output {
        self.checked_mul(rhs)
            .expect("overflow when multiplying duration by scalar")
    }
}

impl MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs
    }
}

impl Div<u32> for Duration {
    type Output = Duration;
    fn div(self, rhs: u32) -> Duration {
        self.checked_div(rhs)
            .expect("divide by zero error when dividing duration by scalar")
    }
}

impl DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs
    }
}

impl Sum<Self> for Duration {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        let mut total = Self::ZERO;
        for d in iter {
            total += d;
        }
        return total;
    }
}

impl<'a> Sum<&'a Self> for Duration {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        let mut total = Self::ZERO;
        for d in iter {
            total += *d;
        }
        return total;
    }
}

static TICK_RATE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn init_time(hwinfo: &crate::hwinfo::HwInfo) {
    TICK_RATE.store(hwinfo.timebase_freq, Ordering::Relaxed);
}

pub struct Instant {
    tick: u64,
}
