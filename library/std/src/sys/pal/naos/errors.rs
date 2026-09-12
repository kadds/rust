//! Generated NaOS error-mapping tables (USERSPACE_FILESYSTEM_PRD §5.3 item 7,
//! §9 last row).
//!
//! Provenance: the two tables below are derived from the NaoIDL generator
//! manifest, which is the single source of truth for error mapping:
//!
//! - `STATUS_*` values: `idl/naoidl.py` / `naos/include/naos/abi.h`
//!   (`NA_STATUS_*`, frozen), mirrored by `naos-sys` and mlibc
//!   `status_errno()`.
//! - `ERRNO_*` values: `ERRNO_VALUES` in `idl/naoidl.py`; typed protocol
//!   failures carry the negated POSIX errno in `ResultFrame.protocol_error`.
//! - Manifests: File rev4 schema hash
//!   `7fc98db22eaac8eea02c2b6e27d6d07fb55976e7a27ce2d9f5d716adea8d3c8e`,
//!   Directory rev2 schema hash
//!   `e7c0e107a9f59254084f99d9af8db6358ae2059d1b6877c01a4e8b10b490eb7a`
//!   (`python3 idl/naoidl.py generate-rust ...`, abi.json side output).
//!
//! This module is intentionally free of `crate::` dependencies so it can be
//! unit-tested on the host with a plain `rustc --test` invocation.

#![allow(dead_code)]

/// NaOS transport status codes (abi.h `NA_STATUS_*`).
pub mod status {
    pub const OK: u32 = 0;
    pub const INVALID_HANDLE: u32 = 1;
    pub const WRONG_BINDING: u32 = 2;
    pub const WRONG_SCOPE: u32 = 3;
    pub const ACCESS_DENIED: u32 = 4;
    pub const INVALID_ARGUMENT: u32 = 5;
    pub const INVALID_MESSAGE: u32 = 6;
    pub const BUFFER_TOO_SMALL: u32 = 7;
    pub const WOULD_BLOCK: u32 = 8;
    pub const WAIT_TIMED_OUT: u32 = 9;
    pub const RESOURCE_EXHAUSTED: u32 = 10;
    pub const FAULT: u32 = 11;
    pub const OBJECT_REVOKED: u32 = 12;
    pub const PEER_CLOSED: u32 = 13;
    pub const ALREADY_CONSUMED: u32 = 14;
    pub const NOT_SUPPORTED: u32 = 15;
    pub const IO_ERROR: u32 = 16;
}

/// POSIX errno values from `ERRNO_VALUES` in `idl/naoidl.py`. Typed protocol
/// failures arrive as `-errno` in `ResultFrame.protocol_error`.
pub mod errno {
    pub const EPERM: i32 = 1;
    pub const ENOENT: i32 = 2;
    pub const ESRCH: i32 = 3;
    pub const EINTR: i32 = 4;
    pub const EIO: i32 = 5;
    pub const ENXIO: i32 = 6;
    pub const E2BIG: i32 = 7;
    pub const ENOEXEC: i32 = 8;
    pub const EBADF: i32 = 9;
    pub const ECHILD: i32 = 10;
    pub const EAGAIN: i32 = 11;
    pub const ENOMEM: i32 = 12;
    pub const EACCES: i32 = 13;
    pub const EFAULT: i32 = 14;
    pub const ENOTBLK: i32 = 15;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const EXDEV: i32 = 18;
    pub const ENODEV: i32 = 19;
    pub const ENOTDIR: i32 = 20;
    pub const EISDIR: i32 = 21;
    pub const EINVAL: i32 = 22;
    pub const ENFILE: i32 = 23;
    pub const EMFILE: i32 = 24;
    pub const ENOTTY: i32 = 25;
    pub const EFBIG: i32 = 27;
    pub const ENOSPC: i32 = 28;
    pub const ESPIPE: i32 = 29;
    pub const EROFS: i32 = 30;
    pub const EMLINK: i32 = 31;
    pub const EPIPE: i32 = 32;
    pub const EDOM: i32 = 33;
    pub const ERANGE: i32 = 34;
    pub const EDEADLK: i32 = 35;
    pub const ENAMETOOLONG: i32 = 36;
    pub const ENOSYS: i32 = 38;
    pub const ENOTEMPTY: i32 = 39;
    pub const ELOOP: i32 = 40;
    pub const EOVERFLOW: i32 = 75;
    pub const EPROTO: i32 = 71;
    pub const EMSGSIZE: i32 = 90;
    pub const ENOTSUP: i32 = 95;
    pub const ETIMEDOUT: i32 = 110;
}

/// Reduced mirror of `crate::io::ErrorKind`. `sys::io::error::naos`
/// converts these to the real `ErrorKind` one-to-one; keeping the mapping
/// local lets this file be unit-tested on the host without compiling std.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Uncategorized,
    NotFound,
    PermissionDenied,
    Interrupted,
    Other,
    ArgumentListTooLong,
    WouldBlock,
    OutOfMemory,
    ResourceBusy,
    AlreadyExists,
    CrossesDevices,
    NotADirectory,
    IsADirectory,
    InvalidInput,
    FileTooLarge,
    StorageFull,
    NotSeekable,
    ReadOnlyFilesystem,
    TooManyLinks,
    BrokenPipe,
    Deadlock,
    InvalidFilename,
    Unsupported,
    DirectoryNotEmpty,
    FilesystemLoop,
    InvalidData,
    TimedOut,
}

/// A raw OS error code on NaOS targets:
/// - `code >= 0`: NaOS transport status (`status::*`).
/// - `code < 0`: negated POSIX errno attached by the serving endpoint
///   (`ResultFrame.protocol_error`), i.e. `-errno::ENOENT == -2`.
///
/// The two spaces share one `raw_os_error: i32` slot; `decode_kind` and
/// `describe` dispatch on the sign. This mirrors mlibc, where transport
/// statuses map through `status_errno()` and typed protocol errors surface
/// as their errno directly.
pub fn decode_kind(code: i32) -> Kind {
    if code < 0 {
        match -code {
            errno::EPERM | errno::EACCES => Kind::PermissionDenied,
            errno::ENOENT => Kind::NotFound,
            errno::EINTR => Kind::Interrupted,
            errno::EIO => Kind::Other,
            errno::E2BIG => Kind::ArgumentListTooLong,
            errno::EAGAIN => Kind::WouldBlock,
            errno::ENOMEM => Kind::OutOfMemory,
            errno::EFAULT => Kind::Other,
            errno::EBUSY => Kind::ResourceBusy,
            errno::EEXIST => Kind::AlreadyExists,
            errno::EXDEV => Kind::CrossesDevices,
            errno::ENOTDIR => Kind::NotADirectory,
            errno::EISDIR => Kind::IsADirectory,
            errno::EINVAL => Kind::InvalidInput,
            errno::EFBIG => Kind::FileTooLarge,
            errno::ENOSPC => Kind::StorageFull,
            errno::ESPIPE => Kind::NotSeekable,
            errno::EROFS => Kind::ReadOnlyFilesystem,
            errno::EMLINK => Kind::TooManyLinks,
            errno::EPIPE => Kind::BrokenPipe,
            errno::EDEADLK => Kind::Deadlock,
            errno::ENAMETOOLONG => Kind::InvalidFilename,
            errno::ENOSYS => Kind::Unsupported,
            errno::ENOTEMPTY => Kind::DirectoryNotEmpty,
            errno::ELOOP => Kind::FilesystemLoop,
            errno::EPROTO | errno::EMSGSIZE => Kind::InvalidData,
            errno::ENOTSUP => Kind::Unsupported,
            errno::ETIMEDOUT => Kind::TimedOut,
            _ => Kind::Other,
        }
    } else {
        // Transport statuses map through mlibc's `status_errno()` and then
        // through the errno table above, so all three handwritten tables
        // (abi.h, mlibc, std) collapse into one mapping.
        match code as u32 {
            status::INVALID_HANDLE => Kind::Uncategorized, // EBADF
            status::WRONG_BINDING => Kind::Uncategorized,  // ENOTTY
            status::WRONG_SCOPE | status::INVALID_ARGUMENT | status::INVALID_MESSAGE => {
                Kind::InvalidInput // EINVAL
            }
            status::ACCESS_DENIED => Kind::PermissionDenied,          // EACCES
            status::BUFFER_TOO_SMALL => Kind::Uncategorized,          // EOVERFLOW
            status::WOULD_BLOCK => Kind::WouldBlock,                  // EAGAIN
            status::WAIT_TIMED_OUT => Kind::TimedOut,                 // ETIMEDOUT
            status::RESOURCE_EXHAUSTED => Kind::OutOfMemory,          // ENOMEM
            status::FAULT => Kind::Other,                             // EFAULT
            status::OBJECT_REVOKED | status::IO_ERROR => Kind::Other, // EIO
            status::PEER_CLOSED => Kind::BrokenPipe,                  // EPIPE
            status::ALREADY_CONSUMED => Kind::Uncategorized,          // EALREADY
            status::NOT_SUPPORTED => Kind::Unsupported,               // ENOTSUP
            _ => Kind::Other,
        }
    }
}

/// Human-readable description for a raw code in either space.
pub fn describe(code: i32) -> &'static str {
    if code < 0 {
        match -code {
            errno::EPERM => "operation not permitted",
            errno::ENOENT => "no such file or directory",
            errno::EIO => "I/O error",
            errno::EBADF => "bad file descriptor",
            errno::EAGAIN => "resource temporarily unavailable",
            errno::ENOMEM => "cannot allocate memory",
            errno::EACCES => "permission denied",
            errno::EFAULT => "bad address",
            errno::EBUSY => "resource busy",
            errno::EEXIST => "file exists",
            errno::EXDEV => "invalid cross-device link",
            errno::ENOTDIR => "not a directory",
            errno::EISDIR => "is a directory",
            errno::EINVAL => "invalid argument",
            errno::EFBIG => "file too large",
            errno::ENOSPC => "no space left on device",
            errno::ESPIPE => "illegal seek",
            errno::EROFS => "read-only file system",
            errno::EPIPE => "broken pipe",
            errno::ENAMETOOLONG => "file name too long",
            errno::ENOSYS => "function not implemented",
            errno::ENOTEMPTY => "directory not empty",
            errno::ELOOP => "too many levels of symbolic links",
            errno::EOVERFLOW => "value too large to store in data type",
            errno::ENOTSUP => "operation not supported",
            errno::ETIMEDOUT => "connection timed out",
            _ => "unknown NaOS protocol error",
        }
    } else {
        match code as u32 {
            status::INVALID_HANDLE => "invalid handle",
            status::WRONG_BINDING => "wrong handle binding",
            status::WRONG_SCOPE => "wrong capability scope",
            status::ACCESS_DENIED => "access denied",
            status::INVALID_ARGUMENT => "invalid argument",
            status::INVALID_MESSAGE => "invalid message",
            status::BUFFER_TOO_SMALL => "buffer too small",
            status::WOULD_BLOCK => "operation would block",
            status::WAIT_TIMED_OUT => "wait timed out",
            status::RESOURCE_EXHAUSTED => "resource exhausted",
            status::FAULT => "memory or capability fault",
            status::OBJECT_REVOKED => "object revoked",
            status::PEER_CLOSED => "peer closed",
            status::ALREADY_CONSUMED => "invocation already consumed",
            status::NOT_SUPPORTED => "operation not supported",
            status::IO_ERROR => "I/O error",
            _ => "unknown NaOS status",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_statuses_map_by_status_table() {
        assert_eq!(decode_kind(status::PEER_CLOSED as i32), Kind::BrokenPipe);
        assert_eq!(decode_kind(status::NOT_SUPPORTED as i32), Kind::Unsupported);
        assert_eq!(decode_kind(status::WOULD_BLOCK as i32), Kind::WouldBlock);
        assert_eq!(decode_kind(status::ACCESS_DENIED as i32), Kind::PermissionDenied);
        assert_eq!(decode_kind(9999), Kind::Other);
    }

    #[test]
    fn protocol_errnos_map_by_errno_table() {
        assert_eq!(decode_kind(-errno::ENOENT), Kind::NotFound);
        assert_eq!(decode_kind(-errno::EEXIST), Kind::AlreadyExists);
        assert_eq!(decode_kind(-errno::ENOTDIR), Kind::NotADirectory);
        assert_eq!(decode_kind(-errno::EISDIR), Kind::IsADirectory);
        assert_eq!(decode_kind(-errno::EXDEV), Kind::CrossesDevices);
        assert_eq!(decode_kind(-errno::ELOOP), Kind::FilesystemLoop);
        assert_eq!(decode_kind(-errno::ENOTEMPTY), Kind::DirectoryNotEmpty);
        assert_eq!(decode_kind(-errno::ENAMETOOLONG), Kind::InvalidFilename);
        assert_eq!(decode_kind(-errno::EFBIG), Kind::FileTooLarge);
        assert_eq!(decode_kind(-errno::ENOSYS), Kind::Unsupported);
        assert_eq!(decode_kind(-errno::ETIMEDOUT), Kind::TimedOut);
        assert_eq!(decode_kind(-errno::ENOMEM), Kind::OutOfMemory);
        assert_eq!(decode_kind(-12345), Kind::Other);
    }

    #[test]
    fn descriptions_cover_both_spaces() {
        assert!(!describe(status::PEER_CLOSED as i32).is_empty());
        assert!(!describe(-errno::ENOENT).is_empty());
        assert_eq!(describe(-errno::ENOENT), "no such file or directory");
    }
}
