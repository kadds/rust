#![stable(feature = "rust1", since = "1.0.0")]

//! NaOS-specific extensions to `std::fs::Metadata`, mirroring the shape of
//! `std::os::unix::fs::MetadataExt` for the fields the NaOS `Stat` payload
//! carries (USERSPACE_FILESYSTEM_PRD §5.2).

use crate::fs::Metadata;
use crate::io;
use crate::path::Path;
use crate::sys::AsInner;

/// Creates a symbolic link at `link` pointing to `original`.
/// Maps to the native `Directory.symlink` method.
#[stable(feature = "rust1", since = "1.0.0")]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    crate::sys::fs::symlink(original.as_ref(), link.as_ref())
}

/// NaOS-specific extensions to [`Metadata`].
#[stable(feature = "rust1", since = "1.0.0")]
pub trait MetadataExt {
    /// Returns the ID of the device containing the file (mount-derived).
    #[stable(feature = "rust1", since = "1.0.0")]
    fn dev(&self) -> u64;
    /// Returns the inode number.
    #[stable(feature = "rust1", since = "1.0.0")]
    fn ino(&self) -> u64;
    /// Returns the number of hard links.
    #[stable(feature = "rust1", since = "1.0.0")]
    fn nlink(&self) -> u64;
    /// Returns the raw st_mode bits (type + permissions).
    #[stable(feature = "rust1", since = "1.0.0")]
    fn mode(&self) -> u32;
    /// Returns the file size in bytes.
    #[stable(feature = "rust1", since = "1.0.0")]
    fn size(&self) -> u64;
}

#[stable(feature = "rust1", since = "1.0.0")]
impl MetadataExt for Metadata {
    fn dev(&self) -> u64 {
        self.as_inner().dev()
    }
    fn ino(&self) -> u64 {
        self.as_inner().inode()
    }
    fn nlink(&self) -> u64 {
        self.as_inner().links()
    }
    fn mode(&self) -> u32 {
        self.as_inner().raw_mode()
    }
    fn size(&self) -> u64 {
        self.as_inner().size()
    }
}
