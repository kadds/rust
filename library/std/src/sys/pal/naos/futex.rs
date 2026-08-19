use crate::sync::atomic::Atomic;
use crate::sync::atomic::Ordering::Relaxed;
use crate::time::Duration;
use core::ptr;

pub type Futex = Atomic<Primitive>;
pub type Primitive = u32;
pub type SmallFutex = Atomic<SmallPrimitive>;
pub type SmallPrimitive = u32;

const FUTEX_WAKE: i32 = 1;
const FUTEX_WAIT: i32 = 2;
const ETIMEDOUT: i32 = 110;
const EINTR: i32 = 4;
const MAX_FUTEX_SECONDS: u64 = u64::MAX / 1_000_000;
const MAX_FUTEX_NANOSECONDS: u64 = (u64::MAX % 1_000_000) * 1_000;

#[repr(C)]
struct NativeTime {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn _s_futex(pointer: *mut i32, operation: i32, value: i32, timeout: *const NativeTime) -> i32;
}

fn relative_timeout(duration: Duration) -> NativeTime {
    let max_seconds = MAX_FUTEX_SECONDS;
    if duration.as_secs() >= max_seconds {
        return NativeTime {
            tv_sec: max_seconds as i64,
            tv_nsec: MAX_FUTEX_NANOSECONDS as i64,
        };
    }
    let mut seconds = duration.as_secs() as i64;
    let nanoseconds = i64::from(duration.subsec_nanos());
    let rounded = ((nanoseconds + 999) / 1_000) * 1_000;
    if rounded == 1_000_000_000 {
        seconds += 1;
        NativeTime { tv_sec: seconds, tv_nsec: 0 }
    } else {
        NativeTime { tv_sec: seconds, tv_nsec: rounded }
    }
}

pub fn futex_wait(futex: &Atomic<u32>, expected: u32, timeout: Option<Duration>) -> bool {
    let timeout = timeout.map(relative_timeout);
    loop {
        if futex.load(Relaxed) != expected {
            return true;
        }
        let result = unsafe {
            _s_futex(
                futex as *const Atomic<u32> as *mut i32,
                FUTEX_WAIT,
                expected as i32,
                timeout
                    .as_ref()
                    .map_or(ptr::null(), |value| value as *const NativeTime),
            )
        };
        if result == -ETIMEDOUT || result == ETIMEDOUT {
            return false;
        }
        if result == -EINTR || result == EINTR {
            continue;
        }
        return true;
    }
}

pub fn futex_wake(futex: &Atomic<u32>) -> bool {
    unsafe {
        _s_futex(
            futex as *const Atomic<u32> as *mut i32,
            FUTEX_WAKE,
            1,
            ptr::null(),
        ) > 0
    }
}

pub fn futex_wake_all(futex: &Atomic<u32>) {
    unsafe {
        _s_futex(
            futex as *const Atomic<u32> as *mut i32,
            FUTEX_WAKE,
            i32::MAX,
            ptr::null(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::relative_timeout;
    use crate::time::Duration;

    #[test]
    fn timeout_rounding_carries_into_seconds() {
        let timeout = relative_timeout(Duration::new(1, 999_999_500));
        assert_eq!(timeout.tv_sec, 2);
        assert_eq!(timeout.tv_nsec, 0);
    }
}
