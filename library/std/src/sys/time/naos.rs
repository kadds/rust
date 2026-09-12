use crate::time::Duration;

#[repr(C)]
struct NativeTime {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn __naos_runtime_clock(clock_index: i32, clock: *mut NativeTime) -> i32;
}

fn now(clock_index: i32) -> Duration {
    let mut clock = NativeTime { tv_sec: 0, tv_nsec: 0 };
    if unsafe { __naos_runtime_clock(clock_index, &mut clock) } != 0
        || clock.tv_sec < 0
        || !(0..1_000_000_000).contains(&clock.tv_nsec)
    {
        crate::sys::pal::abort_internal()
    }
    Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32)
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

impl Instant {
    pub fn now() -> Instant {
        Instant(now(1))
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::ZERO);

impl SystemTime {
    pub const MAX: SystemTime = SystemTime(Duration::MAX);
    pub const MIN: SystemTime = SystemTime(Duration::ZERO);

    /// Builds a timestamp from a stat-style split epoch time. Negative
    /// seconds (pre-epoch) saturate at the epoch because this platform's
    /// `SystemTime` is an unsigned offset from `UNIX_EPOCH`.
    pub fn from_epoch(seconds: i64, nanoseconds: i64) -> SystemTime {
        let seconds = if seconds < 0 { 0 } else { seconds as u64 };
        let extra = if nanoseconds < 0 { 0 } else { nanoseconds as u64 };
        SystemTime(Duration::from_secs(seconds).saturating_add(Duration::from_nanos(extra)))
    }

    pub fn now() -> SystemTime {
        SystemTime(now(0))
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}
