use crate::ffi::OsStr;
use crate::io;
use crate::path::{Path, PathBuf, Prefix};
use crate::sys::unsupported;

path_separator_bytes!(b'/');

#[inline]
pub const fn is_verbatim_sep(b: u8) -> bool {
    is_sep_byte(b)
}

#[inline]
pub fn parse_prefix(_: &OsStr) -> Option<Prefix<'_>> {
    None
}

pub const HAS_PREFIXES: bool = false;

pub(crate) fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.has_root() {
        Ok(path.to_path_buf())
    } else {
        Ok(crate::sys::paths::getcwd()?.join(path))
    }
}

pub(crate) fn is_absolute(path: &Path) -> bool {
    path.has_root()
}
