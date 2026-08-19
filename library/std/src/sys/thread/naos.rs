use crate::ffi::CStr;
use crate::io;
use crate::num::NonZero;
use crate::thread::ThreadInit;
use crate::time::Duration;
use crate::{boxed::Box, mem::ManuallyDrop, ptr};

pub struct Thread {
    handle: *mut u8,
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

pub const DEFAULT_MIN_STACK_SIZE: usize = 0;

unsafe extern "C" {
    fn __naos_runtime_thread_spawn(
        entry: extern "C" fn(*mut u8) -> i64,
        argument: *mut u8,
        output: *mut *mut u8,
    ) -> i32;
    fn __naos_runtime_thread_join(handle: *mut u8) -> i64;
    fn __naos_runtime_thread_detach(handle: *mut u8);
    fn __naos_runtime_current_tid() -> i64;
    fn __naos_runtime_sleep(seconds: i64, nanoseconds: i64) -> i32;
    fn __naos_runtime_yield() -> i32;
}

impl Thread {
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        if stack != 0 {
            return Err(io::Error::UNSUPPORTED_PLATFORM);
        }
        let data = Box::into_raw(init) as *mut u8;
        let mut handle = ptr::null_mut();
        if unsafe { __naos_runtime_thread_spawn(thread_start, data, &mut handle) } != 0 {
            unsafe { drop(Box::from_raw(data as *mut ThreadInit)) };
            return Err(io::Error::new(io::ErrorKind::Other, "unable to create NaOS thread"));
        }
        Ok(Thread { handle })
    }

    pub fn join(self) {
        let this = ManuallyDrop::new(self);
        let result = unsafe { __naos_runtime_thread_join(this.handle) };
        assert_eq!(result, 0, "failed to join NaOS thread");
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { __naos_runtime_thread_detach(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

extern "C" fn thread_start(data: *mut u8) -> i64 {
    let init = unsafe { Box::from_raw(data as *mut ThreadInit) };
    let rust_start = init.init();
    rust_start();
    unsafe { crate::sys::thread_local::destructors::run() };
    crate::rt::thread_cleanup();
    0
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    Err(io::Error::UNSUPPORTED_PLATFORM)
}

pub fn current_os_id() -> Option<u64> {
    let tid = unsafe { __naos_runtime_current_tid() };
    (tid >= 0).then_some(tid as u64)
}

pub fn yield_now() {
    let _ = unsafe { __naos_runtime_yield() };
}

pub fn set_name(_name: &CStr) {}

pub fn sleep(duration: Duration) {
    let Ok(mut seconds) = i64::try_from(duration.as_secs()) else {
        crate::sys::pal::abort_internal()
    };
    let mut nanoseconds = i64::from(duration.subsec_nanos());
    if nanoseconds % 1_000 > 0 {
        nanoseconds = (nanoseconds / 1_000 + 1) * 1_000;
    }
    if nanoseconds == 1_000_000_000 {
        let Some(next_seconds) = seconds.checked_add(1) else {
            crate::sys::pal::abort_internal()
        };
        seconds = next_seconds;
        nanoseconds = 0;
    }
    if unsafe { __naos_runtime_sleep(seconds, nanoseconds) } != 0 {
        crate::sys::pal::abort_internal()
    }
}
