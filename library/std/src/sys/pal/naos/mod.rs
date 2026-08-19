use crate::io;

pub mod unsupported {
    use crate::io;

    pub fn unsupported<T>() -> io::Result<T> {
        Err(unsupported_err())
    }

    pub fn unsupported_err() -> io::Error {
        io::Error::UNSUPPORTED_PLATFORM
    }
}

pub mod futex;

pub use unsupported::{unsupported, unsupported_err};

pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {}
pub unsafe fn cleanup() {}

pub fn abort_internal() -> ! {
    unsafe { __naos_runtime_exit(134) }
}

pub unsafe fn exit(code: i32) -> ! {
    unsafe { __naos_runtime_exit(code as i64) }
}

pub fn getpid() -> u32 {
    let pid = unsafe { __naos_runtime_current_pid() };
    assert!(pid >= 0 && pid <= u32::MAX as i64, "invalid NaOS process id");
    pid as u32
}

unsafe extern "C" {
    fn __naos_runtime_exit(code: i64) -> !;
    fn __naos_runtime_current_pid() -> i64;
}
