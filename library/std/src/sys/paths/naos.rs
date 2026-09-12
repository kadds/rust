//! NaOS `env::current_dir` / `env::set_current_dir` on the runtime-owned
//! namespace bindings (`sys::fs::naos`).

use crate::ffi::{OsStr, OsString};
use crate::marker::PhantomData;
use crate::path::{Path, PathBuf};
use crate::sys::fs::naos as fs_imp;
use crate::{fmt, io};

pub fn getcwd() -> io::Result<PathBuf> {
    let bytes = fs_imp::current_dir_path()?;
    // NaOS paths are opaque byte sequences (same convention as sys::env).
    Ok(PathBuf::from(unsafe { OsString::from_encoded_bytes_unchecked(bytes) }))
}

pub fn chdir(path: &Path) -> io::Result<()> {
    fs_imp::chdir(path)
}

pub struct SplitPaths<'a>(!, PhantomData<&'a ()>);

pub fn split_paths(_unparsed: &OsStr) -> SplitPaths<'_> {
    panic!("unsupported")
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;
    fn next(&mut self) -> Option<PathBuf> {
        self.0
    }
}

#[derive(Debug)]
pub struct JoinPathsError;

pub fn join_paths<I, T>(_paths: I) -> Result<OsString, JoinPathsError>
where
    I: Iterator<Item = T>,
    T: AsRef<OsStr>,
{
    Err(JoinPathsError)
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "not supported on this platform yet".fmt(f)
    }
}

impl crate::error::Error for JoinPathsError {}

pub fn current_exe() -> io::Result<PathBuf> {
    // NaOS exposes the executable as a MemoryObject binding, not a path.
    Err(io::const_error!(io::ErrorKind::Unsupported, "current_exe is not supported on NaOS"))
}

pub fn temp_dir() -> PathBuf {
    PathBuf::from("/tmp")
}

pub fn home_dir() -> Option<PathBuf> {
    None
}
