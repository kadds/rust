//! NaOS implementation of `std::fs` on top of the vendored IDL client
//! (`sys::pal::naos::protocol`) and the refcounted fd table
//! (`sys::fd::naos`). USERSPACE_FILESYSTEM_PRD §5.2/§5.3 item 6-7.
//!
//! All path operations target the process namespace: a root/current
//! directory binding captured at startup (`init_namespace`), mirrored from
//! mlibc's runtime-owned bindings. Paths are passed verbatim to the serving
//! endpoint, which resolves them within the binding -- exactly like mlibc.
//!
//! Wire encodings are byte-identical to mlibc sysdeps:
//! - `open`: mode bits 1|2 read/write, 8 append, 16 nonblock, 128 excl;
//!   attributes 1 creat, 16 directory, 32 file, 256 trunc, 2048 append,
//!   4096 excl.
//! - stat/lstat use `Directory.stat_node` (follow vs NOFOLLOW) instead of
//!   the retired open+stat simulation; rename/link use `rename_at`/
//!   `link_at` with a MOVE'd clone_binding temporary.
//! - dup() of File/Directory descriptors shares one native handle via the
//!   fd table's reference counting; `_na_handle_duplicate` is never called
//!   on those bindings.

use crate::collections::VecDeque;
use crate::ffi::OsString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Path, PathBuf};
use crate::sync::{Mutex, OnceLock, PoisonError};
use crate::sys::fd as fd_table;
use crate::sys::pal::naos::errors::errno;
use crate::sys::pal::naos::protocol::{self, CallError, Handle, Stat};
use crate::sys::time::SystemTime;
use crate::vec::Vec;

// POSIX file-type bits carried in Stat.mode.
const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;

fn os_error(error: CallError) -> io::Error {
    io::Error::from_raw_os_error(error.raw())
}

fn unsupported_err<T>() -> io::Result<T> {
    Err(io::Error::from_raw_os_error(-(errno::ENOSYS as i32)))
}

fn not_supported() -> io::Error {
    io::Error::from_raw_os_error(-(errno::ENOTSUP as i32))
}

/// NUL-terminated encoding for `bytes<4096>` path fields without an explicit
/// length field (open/create/remove/access).
fn path_nul(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = path.as_os_str().as_encoded_bytes().to_vec();
    if bytes.contains(&0) || bytes.len() + 1 > protocol::MAX_PATH_BYTES + 1 {
        return Err(io::Error::from_raw_os_error(-(errno::ENAMETOOLONG as i32)));
    }
    bytes.push(0);
    Ok(bytes)
}

/// Raw encoding for explicit-length path fields (`stat_node`,
/// `rename_at`, ...): no NUL terminator.
fn path_raw(path: &Path) -> io::Result<Vec<u8>> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.contains(&0) || bytes.len() > protocol::MAX_PATH_BYTES {
        return Err(io::Error::from_raw_os_error(-(errno::ENAMETOOLONG as i32)));
    }
    Ok(bytes.to_vec())
}

fn bytes_to_os_string(bytes: Vec<u8>) -> OsString {
    // NaOS paths are opaque byte sequences (same convention as sys::env).
    unsafe { OsString::from_encoded_bytes_unchecked(bytes) }
}

// ---------------------------------------------------------------------------
// Namespace: root/cwd bindings captured at startup
// ---------------------------------------------------------------------------

struct Namespace {
    root: fd_table::FdEntry,
    cwd: fd_table::FdEntry,
}

static NAMESPACE: OnceLock<Mutex<Namespace>> = OnceLock::new();

/// Installs the bootstrap root/current directory bindings. Called once from
/// platform startup (`std::os::naos::init_fs`) with MOVE'd handles.
pub fn init_namespace(root: Handle, cwd: Handle) -> io::Result<()> {
    let namespace = Namespace {
        root: fd_table::FdEntry::from_handle(root)?,
        cwd: fd_table::FdEntry::from_handle(cwd)?,
    };
    NAMESPACE.set(Mutex::new(namespace)).map_err(|_| {
        io::const_error!(io::ErrorKind::AlreadyExists, "NaOS namespace already initialized")
    })
}

fn namespace() -> io::Result<&'static Mutex<Namespace>> {
    NAMESPACE.get().ok_or_else(|| {
        io::const_error!(io::ErrorKind::Unsupported, "NaOS namespace not initialized")
    })
}

fn lock_namespace() -> io::Result<crate::sync::MutexGuard<'static, Namespace>> {
    Ok(namespace()?.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Runs `f` with the current-directory binding handle.
pub fn with_cwd<R>(f: impl FnOnce(Handle) -> Result<R, CallError>) -> io::Result<R> {
    let entry = lock_namespace()?.cwd.clone();
    f(entry.raw()).map_err(os_error)
}

/// Replaces the cwd binding with a freshly opened directory binding
/// (`env::set_current_dir`).
pub fn chdir(path: &Path) -> io::Result<()> {
    let raw = open_directory(path)?;
    let entry = fd_table::FdEntry::from_handle(raw)?;
    lock_namespace()?.cwd = entry;
    Ok(())
}

/// fchdir: the fd's binding is shared, never duplicated (PRD §5.3 item 4).
pub fn fchdir(fd: i32) -> io::Result<()> {
    let entry = fd_table::get_entry(fd)?;
    if !entry.is_shared() {
        return Err(io::const_error!(io::ErrorKind::InvalidInput, "not a directory binding"));
    }
    lock_namespace()?.cwd = entry;
    Ok(())
}

/// chroot per PRD §5.3 item 4 / §6.3: one CHROOT-flagged open returns a
/// restricted binding that replaces BOTH root and cwd; same-process
/// root/cwd share that single binding.
pub fn chroot(path: &Path) -> io::Result<()> {
    let path_bytes = path_nul(path)?;
    let raw = with_cwd(|cwd| {
        protocol::dir_open(
            cwd,
            protocol::OPEN_MODE_READ,
            protocol::NA_DIRECTORY_OPEN_FLAG_CHROOT | protocol::OPEN_ATTR_DIRECTORY,
            &path_bytes,
        )
    })?;
    let entry = fd_table::FdEntry::from_handle(raw)?;
    // Directory bindings are refcount-shared; cloning the entry shares the
    // same native handle between root and cwd.
    let root_clone = entry.clone();
    let mut guard = lock_namespace()?;
    guard.root = root_clone;
    guard.cwd = entry;
    Ok(())
}

/// Absolute path of the current directory via `Directory.path`.
pub fn current_dir_path() -> io::Result<Vec<u8>> {
    with_cwd(protocol::dir_path)
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn open_directory(path: &Path) -> io::Result<Handle> {
    let path_bytes = path_nul(path)?;
    with_cwd(|cwd| {
        protocol::dir_open(
            cwd,
            protocol::OPEN_MODE_READ,
            protocol::OPEN_ATTR_DIRECTORY | protocol::OPEN_ATTR_FILE,
            &path_bytes,
        )
    })
}

/// Translates std `OpenOptions` into the native mode/attribute pair using
/// mlibc's exact encoding (mlibc `open_path_at`).
fn translate_options(opts: &OpenOptions) -> (u64, u64) {
    let access = match (opts.read, opts.write || opts.append) {
        (false, true) => protocol::OPEN_MODE_WRITE,
        (true, true) => protocol::OPEN_MODE_READ | protocol::OPEN_MODE_WRITE,
        _ => protocol::OPEN_MODE_READ,
    };
    let mut mode = access;
    let mut attributes = 0;
    if opts.append {
        mode |= protocol::OPEN_MODE_APPEND;
        attributes |= protocol::OPEN_ATTR_APPEND;
    }
    if opts.nonblock {
        mode |= protocol::OPEN_MODE_NONBLOCK;
    }
    if opts.excl {
        mode |= protocol::OPEN_MODE_EXCL;
        attributes |= protocol::OPEN_ATTR_EXCL;
    }
    if opts.directory {
        attributes |= protocol::OPEN_ATTR_DIRECTORY;
    } else if access != protocol::OPEN_MODE_READ
        || opts.create
        || opts.truncate
        || opts.append
        || opts.excl
    {
        attributes |= protocol::OPEN_ATTR_FILE;
    }
    if opts.create {
        attributes |= protocol::OPEN_ATTR_AUTO_CREATE_FILE;
    }
    if opts.truncate {
        attributes |= protocol::OPEN_ATTR_TRUNC;
    }
    (mode, attributes)
}

fn open_path(opts: &OpenOptions, path: &Path) -> io::Result<Handle> {
    let path_bytes = path_nul(path)?;
    let (mode, attributes) = translate_options(opts);
    with_cwd(|cwd| protocol::dir_open(cwd, mode, attributes, &path_bytes))
}

// ---------------------------------------------------------------------------
// Metadata types
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct FileAttr {
    stat: Stat,
}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.stat.size.max(0) as u64
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions(self.stat.mode & 0o7777)
    }

    pub fn file_type(&self) -> FileType {
        FileType(self.stat.mode)
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from_epoch(self.stat.modify_seconds, self.stat.modify_nanoseconds))
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from_epoch(self.stat.access_seconds, self.stat.access_nanoseconds))
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from_epoch(self.stat.change_seconds, self.stat.change_nanoseconds))
    }

    pub(crate) fn inode(&self) -> u64 {
        self.stat.inode
    }

    pub(crate) fn dev(&self) -> u64 {
        self.stat.device
    }

    pub(crate) fn links(&self) -> u64 {
        self.stat.links
    }

    pub(crate) fn raw_mode(&self) -> u32 {
        self.stat.mode
    }
}

impl fmt::Debug for FileAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileAttr")
            .field("size", &self.size())
            .field("permissions", &self.perm())
            .field("file_type", &self.file_type())
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FilePermissions(u32);

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.0 & 0o222 == 0
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.0 &= !0o222;
        } else {
            self.0 |= 0o200;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileType(u32);

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.0 & S_IFMT == S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.0 & S_IFMT == S_IFREG
    }

    pub fn is_symlink(&self) -> bool {
        self.0 & S_IFMT == S_IFLNK
    }
}

impl fmt::Debug for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_dir() {
            "directory"
        } else if self.is_symlink() {
            "symlink"
        } else {
            "file"
        })
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

impl FileTimes {
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

// ---------------------------------------------------------------------------
// OpenOptions / File
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    directory: bool,
    nonblock: bool,
    excl: bool,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions::default()
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }

    pub fn write(&mut self, write: bool) {
        self.write = write;
    }

    pub fn append(&mut self, append: bool) {
        self.append = append;
    }

    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }

    pub fn create(&mut self, create: bool) {
        self.create = create;
    }

    pub fn create_new(&mut self, create_new: bool) {
        self.create = create_new;
        self.excl = create_new;
    }

    /// Opens directories instead of files (`read_dir` internals).
    pub fn directory(&mut self, directory: bool) {
        self.directory = directory;
    }

    pub fn nonblock(&mut self, nonblock: bool) {
        self.nonblock = nonblock;
    }
}

pub struct File(i32);

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let raw = open_path(opts, path)?;
        match fd_table::install_handle(raw) {
            Ok(fd) => Ok(File(fd)),
            Err(error) => {
                protocol::close_handle(raw);
                Err(error)
            }
        }
    }

    pub fn fd(&self) -> i32 {
        self.0
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        fd_table::with_raw(self.0, protocol::file_stat).map(|stat| FileAttr { stat })
    }

    pub fn fsync(&self) -> io::Result<()> {
        fd_table::with_raw(self.0, protocol::file_sync)
    }

    pub fn datasync(&self) -> io::Result<()> {
        // The NaOS File protocol exposes a single durability barrier; both
        // std flavors map to `File.sync` (PRD §5.3 item 6).
        fd_table::with_raw(self.0, protocol::file_sync)
    }

    pub fn lock(&self) -> io::Result<()> {
        unsupported_err()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported_err()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(io::Error::from_raw_os_error(-(errno::ENOSYS as i32))))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        self.try_lock()
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported_err()
    }

    pub fn truncate(&self, size: u64) -> io::Result<()> {
        fd_table::with_raw(self.0, |raw| protocol::file_truncate(raw, size))
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let size = buf.len().min(protocol::MAX_MESSAGE_BYTES) as u64;
        fd_table::with_raw(self.0, |raw| protocol::file_read(raw, size, buf))
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let first = match bufs.iter_mut().find(|buf| !buf.is_empty()) {
            Some(buf) => &mut **buf,
            None => return Ok(0),
        };
        self.read(first)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        crate::io::default_read_buf(|buf| self.read(buf), cursor)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let chunk = &buf[..buf.len().min(protocol::MAX_MESSAGE_BYTES - 32)];
        let written = fd_table::with_raw(self.0, |raw| protocol::file_write(raw, chunk))?;
        Ok(written as usize)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        // Sequential writes keep semantics simple; the server-side cursor
        // makes them equivalent to one vectored request.
        let mut total = 0;
        for buf in bufs {
            if buf.is_empty() {
                continue;
            }
            total += self.write(buf)?;
            if total < buf.len() {
                break;
            }
        }
        Ok(total)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match pos {
            SeekFrom::Start(offset) => (offset as i64, protocol::LSEEK_BEGIN),
            SeekFrom::Current(offset) => (offset, protocol::LSEEK_CURRENT),
            SeekFrom::End(offset) => (offset, protocol::LSEEK_END),
        };
        let result =
            fd_table::with_raw(self.0, |raw| protocol::file_seek(raw, offset, whence))?;
        Ok(result.max(0) as u64)
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        Some(self.file_attr().map(|attr| attr.size()))
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        // Refcount-shared for File bindings (never `_na_handle_duplicate`).
        Ok(File(fd_table::dup(self.0)?))
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        Err(not_supported())
    }

    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        Err(not_supported())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = fd_table::close(self.0);
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("fd", &self.0).finish()
    }
}

// ---------------------------------------------------------------------------
// Directory builder
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct DirBuilder {}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, path: &Path) -> io::Result<()> {
        mkdir(path, 0o777)
    }
}

fn mkdir(path: &Path, mode: u32) -> io::Result<()> {
    let path_bytes = path_nul(path)?;
    with_cwd(|cwd| {
        protocol::dir_create(cwd, mode as u64, protocol::CREATE_FLAG_PRESENT, &path_bytes)
    })
}

// ---------------------------------------------------------------------------
// Directory iteration
// ---------------------------------------------------------------------------

pub struct ReadDir {
    dir: File,
    root: PathBuf,
    cursor: u64,
    pending: VecDeque<(u64, u32, OsString)>,
    finished: bool,
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadDir").field("cursor", &self.cursor).finish()
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        loop {
            if let Some((inode, kind, name)) = self.pending.pop_front() {
                return Some(Ok(DirEntry {
                    root: self.root.clone(),
                    dir_fd: self.dir.fd(),
                    inode,
                    kind,
                    name,
                }));
            }
            if self.finished {
                return None;
            }
            let cursor = self.cursor;
            match fd_table::with_raw(self.dir.fd(), |raw| protocol::dir_list(raw, cursor)) {
                Ok((next, entries)) => {
                    self.cursor = next;
                    self.finished = entries.is_empty();
                    self.pending.extend(entries.into_iter().map(|record| {
                        (
                            record.inode,
                            record.kind,
                            bytes_to_os_string(record.name[..record.name_len].to_vec()),
                        )
                    }));
                }
                Err(error) => {
                    self.finished = true;
                    return Some(Err(error));
                }
            }
        }
    }
}

pub struct DirEntry {
    root: PathBuf,
    dir_fd: i32,
    inode: u64,
    kind: u32,
    name: OsString,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }

    pub fn file_name(&self) -> OsString {
        self.name.clone()
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        self.stat_node(false)
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        // NOFOLLOW query reports the entry's own type (POSIX lstat).
        self.stat_node(true).map(|attr| attr.file_type())
    }

    fn stat_node(&self, nofollow: bool) -> io::Result<FileAttr> {
        let name = path_raw(Path::new(&self.name))?;
        fd_table::with_raw(self.dir_fd, |raw| {
            protocol::dir_stat_node(raw, nofollow, &name)
        })
        .map(|stat| FileAttr { stat })
    }
}

// ---------------------------------------------------------------------------
// Top-level operations (the `imp` surface used by sys::fs)
// ---------------------------------------------------------------------------

pub fn readdir(path: &Path) -> io::Result<ReadDir> {
    let raw = open_directory(path)?;
    let dir = match fd_table::install_handle(raw) {
        Ok(fd) => File(fd),
        Err(error) => {
            protocol::close_handle(raw);
            return Err(error);
        }
    };
    Ok(ReadDir {
        dir,
        root: path.to_path_buf(),
        cursor: 0,
        pending: VecDeque::new(),
        finished: false,
    })
}

pub fn unlink(path: &Path) -> io::Result<()> {
    remove_node(path, 0)
}

fn remove_node(path: &Path, flags: u64) -> io::Result<()> {
    let path_bytes = path_nul(path)?;
    with_cwd(|cwd| protocol::dir_remove(cwd, 0, flags, &path_bytes))
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    let first = path_raw(old)?;
    let second = path_raw(new)?;
    with_cwd(|cwd| protocol::dir_rename_at(cwd, cwd, &first, &second))
}

/// `fs::hard_link` maps to `Directory.link_at` (PRD §5.3 item 6).
pub fn link(original: &Path, link_path: &Path) -> io::Result<()> {
    let first = path_raw(original)?;
    let second = path_raw(link_path)?;
    with_cwd(|cwd| protocol::dir_link_at(cwd, cwd, &first, &second))
}

pub fn set_perm(_path: &Path, perm: FilePermissions) -> io::Result<()> {
    let _ = perm;
    Err(not_supported())
}

pub fn set_times(_path: &Path, _times: FileTimes) -> io::Result<()> {
    Err(not_supported())
}

pub fn set_times_nofollow(_path: &Path, _times: FileTimes) -> io::Result<()> {
    Err(not_supported())
}

pub fn rmdir(path: &Path) -> io::Result<()> {
    remove_node(path, protocol::REMOVE_FLAG_DIRECTORY)
}

pub fn readlink(path: &Path) -> io::Result<PathBuf> {
    let raw = path_raw(path)?;
    let target = with_cwd(|cwd| protocol::dir_readlink(cwd, &raw))?;
    Ok(PathBuf::from(bytes_to_os_string(target)))
}

pub fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    let target = path_raw(original)?;
    let link_path = path_raw(link)?;
    with_cwd(|cwd| protocol::dir_symlink(cwd, &target, &link_path))
}

pub fn stat(path: &Path) -> io::Result<FileAttr> {
    stat_node(path, false)
}

pub fn lstat(path: &Path) -> io::Result<FileAttr> {
    stat_node(path, true)
}

fn stat_node(path: &Path, nofollow: bool) -> io::Result<FileAttr> {
    let raw = path_raw(path)?;
    with_cwd(|cwd| protocol::dir_stat_node(cwd, nofollow, &raw)).map(|stat| FileAttr { stat })
}

pub fn canonicalize(_path: &Path) -> io::Result<PathBuf> {
    // Not part of Phase 3 scope; `Directory.path` covers only whole
    // bindings, per-component canonicalization needs a dedicated method.
    Err(io::Error::from_raw_os_error(-(errno::ENOSYS as i32)))
}

pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    crate::sys::fs::common::remove_dir_all(path)
}

pub fn exists(path: &Path) -> io::Result<bool> {
    crate::sys::fs::common::exists(path)
}

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    crate::sys::fs::common::copy(from, to)
}

// ---------------------------------------------------------------------------
// sys::fs::Dir support (used by std::fs::Dir)
// ---------------------------------------------------------------------------

pub struct Dir {
    inner: File,
}

impl Dir {
    pub fn open(path: &Path, _opts: &OpenOptions) -> io::Result<Self> {
        Ok(Dir { inner: File::open(path, &OpenOptions::new())? })
    }

    pub fn open_file(&self, path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let path_bytes = path_nul(path)?;
        let (mode, attributes) = translate_options(opts);
        let raw = fd_table::with_raw(self.inner.fd(), |dir| {
            protocol::dir_open(dir, mode, attributes, &path_bytes)
        })?;
        match fd_table::install_handle(raw) {
            Ok(fd) => Ok(File(fd)),
            Err(error) => {
                protocol::close_handle(raw);
                Err(error)
            }
        }
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dir").field("fd", &self.inner.fd()).finish()
    }
}
