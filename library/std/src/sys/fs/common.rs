#![allow(dead_code)] // not used on all platforms

use crate::io::{self, Error, ErrorKind};
use crate::path::{Path, PathBuf};
use crate::sys::fs::{File, OpenOptions};
use crate::sys::helpers::ignore_notfound;
use crate::{fmt, fs};

pub(crate) const NOT_FILE_ERROR: Error = io::const_error!(
    ErrorKind::InvalidInput,
    "the source path is neither a regular file nor a symlink to a regular file",
);

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let mut reader = fs::File::open(from)?;
    let metadata = reader.metadata()?;

    if !metadata.is_file() {
        return Err(NOT_FILE_ERROR);
    }

    let mut writer = fs::File::create(to)?;
    let perm = metadata.permissions();

    let ret = io::copy(&mut reader, &mut writer)?;
    writer.set_permissions(perm)?;
    Ok(ret)
}

pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    let filetype = fs::symlink_metadata(path)?.file_type();
    if filetype.is_symlink() { fs::remove_file(path) } else { remove_dir_all_recursive(path) }
}

fn remove_dir_all_recursive(path: &Path) -> io::Result<()> {
    // Snapshot the directory before mutating it. The NaOS Directory.list
    // cursor is name-based; deleting entries while its ReadDir is alive can
    // invalidate the cursor and surface EINVAL on the next page.
    let children: io::Result<Vec<(PathBuf, bool)>> = fs::read_dir(path)?
        .map(|entry| {
            let entry = entry?;
            let is_dir = entry.file_type()?.is_dir();
            Ok((entry.path(), is_dir))
        })
        .collect();
    for (child_path, is_dir) in children? {
        let result: io::Result<()> = if is_dir {
            remove_dir_all_recursive(&child_path)
        } else {
            fs::remove_file(&child_path)
        };
        // ignore internal NotFound errors to prevent race conditions
        if let Err(err) = &result
            && err.kind() != io::ErrorKind::NotFound
        {
            return result;
        }
    }
    ignore_notfound(fs::remove_dir(path))
}

pub fn exists(path: &Path) -> io::Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub struct Dir {
    path: PathBuf,
}

impl Dir {
    pub fn open(path: &Path, _opts: &OpenOptions) -> io::Result<Self> {
        path.canonicalize().map(|path| Self { path })
    }

    pub fn open_file(&self, path: &Path, opts: &OpenOptions) -> io::Result<File> {
        File::open(&self.path.join(path), &opts)
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dir").field("path", &self.path).finish()
    }
}
