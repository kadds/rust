#![stable(feature = "rust1", since = "1.0.0")]

pub mod fs;
pub mod io;

use crate::path::Path;

/// Installs the process filesystem namespace into the standard library.
///
/// `root` and `cwd` are the bootstrap Directory bindings (from
/// `naos_runtime::Bootstrap::root_directory` / `current_directory`). The
/// handles MOVE into std: after this call the caller must not close or use
/// them again (`BootstrapHandle::into_raw` transfers without dropping).
///
/// Every `std::fs` operation resolves paths through these bindings, exactly
/// like mlibc's runtime-owned root/cwd (USERSPACE_FILESYSTEM_PRD §5.3
/// item 4, §9 last row).
#[stable(feature = "rust1", since = "1.0.0")]
pub fn init_fs(root: u64, cwd: u64) -> crate::io::Result<()> {
    crate::sys::fs::naos::init_namespace(root, cwd)
}

/// Replaces both the root and current directory with a CHROOT-flagged
/// restricted binding opened at `path` (PRD §5.3 item 4 / §6.3 semantics).
#[stable(feature = "rust1", since = "1.0.0")]
pub fn chroot<P: AsRef<Path>>(path: P) -> crate::io::Result<()> {
    crate::sys::fs::naos::chroot(path.as_ref())
}
