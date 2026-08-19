use crate::ffi::OsString;
use crate::{fmt, vec};

#[repr(C)]
#[derive(Clone, Copy)]
struct SnapshotSlice {
    pointer: *const u8,
    length: usize,
}

unsafe extern "C" {
    fn __naos_runtime_arg_count() -> usize;
    fn __naos_runtime_arg(index: usize) -> SnapshotSlice;
}

pub struct Args {
    iter: vec::IntoIter<OsString>,
}

impl Args {
    pub fn new() -> Self {
        let count = unsafe { __naos_runtime_arg_count() };
        let mut values = vec::Vec::with_capacity(count);
        for index in 0..count {
            let slice = unsafe { __naos_runtime_arg(index) };
            if slice.pointer.is_null() {
                continue;
            }
            let bytes = unsafe { core::slice::from_raw_parts(slice.pointer, slice.length) }.to_vec();
            values.push(unsafe { OsString::from_encoded_bytes_unchecked(bytes) });
        }
        Self { iter: values.into_iter() }
    }
}

impl fmt::Debug for Args {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter.as_slice()).finish()
    }
}

impl Iterator for Args {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Args {
    fn next_back(&mut self) -> Option<OsString> {
        self.iter.next_back()
    }
}

impl ExactSizeIterator for Args {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub fn args() -> Args {
    Args::new()
}
