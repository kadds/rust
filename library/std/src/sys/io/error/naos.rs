//! NaOS `io::Error` plumbing. Mapping tables live in
//! `sys::pal::naos::errors` (generated from the NaoIDL manifest, the single
//! source of truth per USERSPACE_FILESYSTEM_PRD §5.3 item 7); this module
//! adapts them to std types and preserves raw codes in `raw_os_error`.
//!
//! Raw-code convention (see `sys::pal::naos::errors`):
//! - positive code = NaOS transport status (`NA_STATUS_*`);
//! - negative code = negated POSIX errno from a typed protocol failure.

use crate::io::{ErrorKind, RawOsError};
use crate::sys::pal::naos::errors;

pub fn errno() -> RawOsError {
    0
}

pub fn is_interrupted(code: RawOsError) -> bool {
    code < 0 && -(code as i64) == errors::errno::EINTR as i64
}

fn kind_from_shim(kind: errors::Kind) -> ErrorKind {
    match kind {
        errors::Kind::Uncategorized => ErrorKind::Uncategorized,
        errors::Kind::NotFound => ErrorKind::NotFound,
        errors::Kind::PermissionDenied => ErrorKind::PermissionDenied,
        errors::Kind::Interrupted => ErrorKind::Interrupted,
        errors::Kind::Other => ErrorKind::Other,
        errors::Kind::ArgumentListTooLong => ErrorKind::ArgumentListTooLong,
        errors::Kind::WouldBlock => ErrorKind::WouldBlock,
        errors::Kind::OutOfMemory => ErrorKind::OutOfMemory,
        errors::Kind::ResourceBusy => ErrorKind::ResourceBusy,
        errors::Kind::AlreadyExists => ErrorKind::AlreadyExists,
        errors::Kind::CrossesDevices => ErrorKind::CrossesDevices,
        errors::Kind::NotADirectory => ErrorKind::NotADirectory,
        errors::Kind::IsADirectory => ErrorKind::IsADirectory,
        errors::Kind::InvalidInput => ErrorKind::InvalidInput,
        errors::Kind::FileTooLarge => ErrorKind::FileTooLarge,
        errors::Kind::StorageFull => ErrorKind::StorageFull,
        errors::Kind::NotSeekable => ErrorKind::NotSeekable,
        errors::Kind::ReadOnlyFilesystem => ErrorKind::ReadOnlyFilesystem,
        errors::Kind::TooManyLinks => ErrorKind::TooManyLinks,
        errors::Kind::BrokenPipe => ErrorKind::BrokenPipe,
        errors::Kind::Deadlock => ErrorKind::Deadlock,
        errors::Kind::InvalidFilename => ErrorKind::InvalidFilename,
        errors::Kind::Unsupported => ErrorKind::Unsupported,
        errors::Kind::DirectoryNotEmpty => ErrorKind::DirectoryNotEmpty,
        errors::Kind::FilesystemLoop => ErrorKind::FilesystemLoop,
        errors::Kind::InvalidData => ErrorKind::InvalidData,
        errors::Kind::TimedOut => ErrorKind::TimedOut,
    }
}

pub fn decode_error_kind(code: RawOsError) -> ErrorKind {
    kind_from_shim(errors::decode_kind(code))
}

pub fn error_string(code: RawOsError) -> String {
    errors::describe(code).to_owned()
}
