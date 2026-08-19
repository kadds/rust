use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};

pub struct Stdin;
pub struct Stdout;
pub type Stderr = Stdout;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    #[inline]
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn read_buf(&mut self, _cursor: BorrowedCursor<'_>) -> io::Result<()> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn read_vectored(&mut self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn is_read_vectored(&self) -> bool {
        // Do not force `Chain<Empty, T>` or `Chain<T, Empty>` to use vectored
        // reads, unless the other reader is vectored.
        false
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let _ = buf;
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn read_buf_exact(&mut self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        let _ = cursor;
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn read_to_end(&mut self, _buf: &mut Vec<u8>) -> io::Result<usize> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn read_to_string(&mut self, _buf: &mut String) -> io::Result<usize> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = buf;
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let _ = bufs;
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn write_all(&mut self, _buf: &[u8]) -> io::Result<()> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    #[inline]
    fn write_all_vectored(&mut self, _bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }

    // Keep the default write_fmt so the `fmt::Arguments` are still evaluated.

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::UNSUPPORTED_PLATFORM)
    }
}

pub const STDIN_BUF_SIZE: usize = 0;

pub fn is_ebadf(_err: &io::Error) -> bool {
    true
}

pub fn panic_output() -> Option<Vec<u8>> {
    None
}
