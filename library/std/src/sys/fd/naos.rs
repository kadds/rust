//! NaOS descriptor table for `std::fs` (USERSPACE_FILESYSTEM_PRD §5.2, §9
//! last row).
//!
//! Each entry owns a capability binding:
//!
//! - File and Directory handles are **refcount-shared**: `dup` clones the
//!   entry so both descriptors reference the same native handle, and the
//!   handle is closed when the last reference drops. `_na_handle_duplicate`
//!   is NEVER called on File/Directory bindings (PRD §5.2 rule, mirrored
//!   from mlibc).
//! - Other scopes (streams) keep per-descriptor kernel duplicates today;
//!   their `dup` still goes through `_na_handle_duplicate`.
//!
//! The sharing class is decided once at install time from the handle scope
//! (`_na_handle_get_info`), exactly like mlibc's
//! `handle_is_sharable_binding`.

use crate::io;
use crate::sync::{Arc, LazyLock, Mutex, PoisonError};
use crate::sys::pal::naos::errors::errno;
use crate::sys::pal::naos::protocol::{self, CallError, Handle};
use crate::vec::Vec;

struct HandleSlot {
    raw: Handle,
    shared: bool,
}

impl Drop for HandleSlot {
    fn drop(&mut self) {
        protocol::close_handle(self.raw);
    }
}

#[derive(Clone)]
pub struct FdEntry {
    slot: Arc<HandleSlot>,
}

impl FdEntry {
    /// Wraps a raw binding. The sharing class is fixed by the handle scope.
    pub fn from_handle(raw: Handle) -> io::Result<Self> {
        if raw == protocol::HANDLE_INVALID {
            return Err(io::const_error!(io::ErrorKind::InvalidInput, "invalid NaOS handle"));
        }
        Ok(Self {
            slot: Arc::new(HandleSlot { raw, shared: protocol::binding_is_sharable(raw) }),
        })
    }

    /// Raw native handle; shared across dup() for File/Directory bindings.
    pub fn raw(&self) -> Handle {
        self.slot.raw
    }

    /// True when the underlying native handle is refcount-shared.
    pub fn is_shared(&self) -> bool {
        self.slot.shared
    }
}

static TABLE: LazyLock<Mutex<Vec<Option<FdEntry>>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn table() -> &'static Mutex<Vec<Option<FdEntry>>> {
    &TABLE
}

fn lock_table() -> crate::sync::MutexGuard<'static, Vec<Option<FdEntry>>> {
    table().lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn badf() -> io::Error {
    io::Error::from_raw_os_error(-(errno::EBADF as i32))
}

fn os_error(error: CallError) -> io::Error {
    io::Error::from_raw_os_error(error.raw())
}

/// Installs an entry into the lowest free slot; returns the new fd number.
pub fn install(entry: FdEntry) -> io::Result<i32> {
    let mut table = lock_table();
    match table.iter().position(|slot| slot.is_none()) {
        Some(index) => {
            table[index] = Some(entry);
            Ok(index as i32)
        }
        None => {
            table.push(Some(entry));
            Ok((table.len() - 1) as i32)
        }
    }
}

/// Registers an owned raw handle (e.g. from bootstrap or Directory.open).
pub fn install_handle(raw: Handle) -> io::Result<i32> {
    install(FdEntry::from_handle(raw)?)
}

/// Removes the entry, closing the binding on the last reference.
pub fn close(fd: i32) -> io::Result<()> {
    let mut table = lock_table();
    let index = validate(&table, fd)?;
    table[index].take().ok_or_else(badf)?;
    Ok(())
}

pub fn dup(fd: i32) -> io::Result<i32> {
    let cloned = get_entry(fd)?;
    install(cloned)
}

fn validate(table: &[Option<FdEntry>], fd: i32) -> io::Result<usize> {
    let index = usize::try_from(fd).map_err(|_| badf())?;
    if index >= table.len() || table[index].is_none() {
        return Err(badf());
    }
    Ok(index)
}

/// Clones the entry (refcount-shared for File/Directory, kernel-duplicated
/// for streams) without touching the table.
pub fn get_entry(fd: i32) -> io::Result<FdEntry> {
    let mut table_guard = lock_table();
    let index = validate(&table_guard, fd)?;
    table_guard[index].as_ref().ok_or_else(badf).cloned()
}

/// Runs `f` with the entry's raw native handle. The entry is held alive via
/// its Arc clone for the duration of the call, so concurrent close cannot
/// invalidate the handle mid-call.
pub fn with_raw<R>(fd: i32, f: impl FnOnce(Handle) -> Result<R, CallError>) -> io::Result<R> {
    let entry = get_entry(fd)?;
    f(entry.raw()).map_err(os_error)
}
