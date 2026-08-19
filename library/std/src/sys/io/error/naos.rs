use crate::io::{ErrorKind, RawOsError};

const STATUS_INVALID_HANDLE: u32 = 1;
const STATUS_WRONG_BINDING: u32 = 2;
const STATUS_WRONG_SCOPE: u32 = 3;
const STATUS_ACCESS_DENIED: u32 = 4;
const STATUS_INVALID_ARGUMENT: u32 = 5;
const STATUS_INVALID_MESSAGE: u32 = 6;
const STATUS_BUFFER_TOO_SMALL: u32 = 7;
const STATUS_WOULD_BLOCK: u32 = 8;
const STATUS_WAIT_TIMED_OUT: u32 = 9;
const STATUS_RESOURCE_EXHAUSTED: u32 = 10;
const STATUS_FAULT: u32 = 11;
const STATUS_OBJECT_REVOKED: u32 = 12;
const STATUS_PEER_CLOSED: u32 = 13;
const STATUS_ALREADY_CONSUMED: u32 = 14;
const STATUS_NOT_SUPPORTED: u32 = 15;
const STATUS_IO_ERROR: u32 = 16;

pub fn errno() -> RawOsError {
    0
}

pub fn is_interrupted(_: RawOsError) -> bool {
    false
}

pub fn decode_error_kind(code: RawOsError) -> ErrorKind {
    match code as u32 {
        STATUS_INVALID_HANDLE | STATUS_INVALID_ARGUMENT | STATUS_ALREADY_CONSUMED => {
            ErrorKind::InvalidInput
        }
        STATUS_WRONG_BINDING | STATUS_WRONG_SCOPE | STATUS_ACCESS_DENIED => {
            ErrorKind::PermissionDenied
        }
        STATUS_INVALID_MESSAGE => ErrorKind::InvalidData,
        STATUS_RESOURCE_EXHAUSTED => ErrorKind::ResourceBusy,
        STATUS_FAULT => ErrorKind::Other,
        STATUS_WOULD_BLOCK => ErrorKind::WouldBlock,
        STATUS_BUFFER_TOO_SMALL => ErrorKind::InvalidInput,
        STATUS_WAIT_TIMED_OUT => ErrorKind::TimedOut,
        STATUS_OBJECT_REVOKED | STATUS_PEER_CLOSED => ErrorKind::BrokenPipe,
        STATUS_NOT_SUPPORTED => ErrorKind::Unsupported,
        STATUS_IO_ERROR => ErrorKind::Other,
        _ => ErrorKind::Uncategorized,
    }
}

pub fn error_string(code: RawOsError) -> String {
    match code as u32 {
        STATUS_INVALID_HANDLE => "invalid handle".to_owned(),
        STATUS_WRONG_BINDING => "wrong handle binding".to_owned(),
        STATUS_WRONG_SCOPE => "wrong capability scope".to_owned(),
        STATUS_ACCESS_DENIED => "access denied".to_owned(),
        STATUS_INVALID_ARGUMENT => "invalid argument".to_owned(),
        STATUS_INVALID_MESSAGE => "invalid message".to_owned(),
        STATUS_BUFFER_TOO_SMALL => "buffer too small".to_owned(),
        STATUS_WOULD_BLOCK => "operation would block".to_owned(),
        STATUS_WAIT_TIMED_OUT => "wait timed out".to_owned(),
        STATUS_RESOURCE_EXHAUSTED => "resource exhausted".to_owned(),
        STATUS_FAULT => "memory or capability fault".to_owned(),
        STATUS_OBJECT_REVOKED => "object revoked".to_owned(),
        STATUS_PEER_CLOSED => "peer closed".to_owned(),
        STATUS_ALREADY_CONSUMED => "invocation already consumed".to_owned(),
        STATUS_NOT_SUPPORTED => "operation not supported".to_owned(),
        STATUS_IO_ERROR => "I/O error".to_owned(),
        _ => "unknown NaOS status".to_owned(),
    }
}
