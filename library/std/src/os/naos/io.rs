#![stable(feature = "rust1", since = "1.0.0")]

use crate::io;

/// A raw NaOS transport status preserved by a standard I/O error.
#[stable(feature = "rust1", since = "1.0.0")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusError(u32);

impl StatusError {
    /// Creates a typed status from the native ABI value.
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn new(status: u32) -> Self {
        Self(status)
    }

    /// Returns the original native ABI value.
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn status(self) -> u32 {
        self.0
    }

    /// Extracts the native status from an OS-backed `io::Error`.
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn from_io(error: &io::Error) -> Option<Self> {
        error.raw_os_error().map(|status| Self(status as u32))
    }
}

/// Builds an OS-backed `io::Error` without translating the native status to a
/// Linux errno. `StatusError::from_io` recovers the original `u32` value.
#[stable(feature = "rust1", since = "1.0.0")]
pub fn from_status(status: u32) -> io::Error {
    io::Error::from_raw_os_error(status as i32)
}
