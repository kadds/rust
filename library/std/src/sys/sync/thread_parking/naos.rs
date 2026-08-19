use crate::pin::Pin;
use crate::sync::atomic::AtomicU32;
use crate::sync::atomic::Ordering::{Acquire, Release};
use crate::sys::futex::{futex_wait, futex_wake};
use crate::time::Duration;

const PARKED: u32 = u32::MAX;
const EMPTY: u32 = 0;
const NOTIFIED: u32 = 1;

pub struct Parker {
    state: AtomicU32,
}

impl Parker {
    pub unsafe fn new_in_place(parker: *mut Parker) {
        unsafe { parker.write(Self { state: AtomicU32::new(EMPTY) }) };
    }

    pub unsafe fn park(self: Pin<&Self>) {
        if self.state.fetch_sub(1, Acquire) == NOTIFIED {
            return;
        }
        loop {
            let _ = futex_wait(&self.state, PARKED, None);
            if self.state.compare_exchange(NOTIFIED, EMPTY, Acquire, Acquire).is_ok() {
                return;
            }
        }
    }

    pub unsafe fn park_timeout(self: Pin<&Self>, duration: Duration) {
        if self.state.fetch_sub(1, Acquire) == NOTIFIED {
            return;
        }
        let _ = futex_wait(&self.state, PARKED, Some(duration));

        // Consume a permit that arrived before this return, or retire the
        // parked state if the wait timed out/spuriously woke.  A plain swap
        // could erase a concurrent unpark after the futex returned and lose
        // the permit for the next park.
        loop {
            let state = self.state.load(Acquire);
            if state == NOTIFIED {
                if self.state.compare_exchange(NOTIFIED, EMPTY, Acquire, Acquire).is_ok() {
                    break;
                }
            } else if state == PARKED {
                if self.state.compare_exchange(PARKED, EMPTY, Acquire, Acquire).is_ok() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn unpark(self: Pin<&Self>) {
        if self.state.swap(NOTIFIED, Release) == PARKED {
            let _ = futex_wake(&self.state);
        }
    }
}

unsafe impl Send for Parker {}
unsafe impl Sync for Parker {}
