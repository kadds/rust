//! Compact NaOS IDL client for the File and Directory protocols, used by the
//! `std` PAL fd/fs layers (USERSPACE_FILESYSTEM_PRD §5.3 item 7, §9 last row).
//!
//! Provenance: this module is a manifest-derived client subset of the
//! generated `naos-idl` Rust bindings (`python3 idl/naoidl.py generate-rust`,
//! templates `idl/templates/rust/naoidl/binding.rs.htt`). The upstream crate
//! cannot be linked into `library/std` (the fork builds std in isolation via
//! `x.py`), so the frozen wire contract is vendored here:
//!
//! - File rev4 schema hash
//!   `7fc98db22eaac8eea02c2b6e27d6d07fb55976e7a27ce2d9f5d716adea8d3c8e`
//! - Directory rev2 schema hash
//!   `e7c0e107a9f59254084f99d9af8db6358ae2059d1b6877c01a4e8b10b490eb7a`
//!
//! Wire rules (canonical encoding, identical to the generated encoders):
//! scalar fields serialize little-endian in `@id` order with no padding;
//! `bytes<N>` fields are inline blobs; a trailing blob runs to the end of the
//! message; `client_end`/handle fields are u32 slots into the resource
//! disposition array attached to the submit frame.
//!
//! Semantics mirrored from mlibc `sysdeps/naos`: dup() shares File/Directory
//! native handles by reference counting (`_na_handle_duplicate` is never
//! called on those bindings), `rename_at`/`link_at` clone the new-parent
//! binding and MOVE the temporary copy into the request.

use crate::sys::pal::naos::errors::status;
use crate::vec::Vec;

pub type Handle = u64;
pub type RawStatus = u32;

pub const HANDLE_INVALID: Handle = 0;

// Frozen ABI constants (naos/include/naos/abi.h).
pub const RESOURCE_MOVE: u32 = 1;
pub const RESOURCE_DUPLICATE: u32 = 2;
pub const MEMORY_MAP_READ: u32 = 1 << 0;
pub const MEMORY_MAP_WRITE: u32 = 1 << 1;
pub const MEMORY_MAP_SHARED: u32 = 1 << 3;

/// Handle scopes (`NA_SCOPE_*`). Scope decides whether a descriptor binding
/// is refcount-shared across dup() (File/Directory) or kernel-duplicated
/// (streams and everything else).
pub const SCOPE_FILE: u64 = 2;
pub const SCOPE_DIRECTORY: u64 = 3;

/// Directory UAPI flag bits (abi.h, frozen by USERSPACE_FILESYSTEM_PRD §5.3.6).
pub const NA_DIRECTORY_LOOKUP_FLAG_NOFOLLOW: u64 = 1 << 0;
pub const NA_DIRECTORY_OPEN_FLAG_CHROOT: u64 = 1 << 63;

// Method ordinals -- generated constants, do not edit by hand.
// File rev6 (`METHOD_*` / scope from idl/system/file.naidl).
pub mod file_method {
    pub const PREAD: u64 = 1;
    pub const PWRITE: u64 = 2;
    pub const SEEK: u64 = 3;
    pub const STAT: u64 = 4;
    pub const SYNC: u64 = 5;
    pub const TRUNCATE: u64 = 6;
    pub const READ: u64 = 11;
    pub const WRITE: u64 = 12;
    pub const MATERIALIZE: u64 = 17;
}

// Directory rev2 (Directory.abi.json, schema hash above).
pub mod dir_method {
    pub const OPEN: u64 = 1;
    pub const LIST: u64 = 2;
    pub const STAT: u64 = 3;
    pub const CREATE: u64 = 4;
    pub const REMOVE: u64 = 5;
    pub const PATH: u64 = 6;
    pub const SYMLINK: u64 = 10;
    pub const READLINK: u64 = 11;
    pub const CLONE_BINDING: u64 = 14;
    pub const STAT_NODE: u64 = 15;
    pub const SYNC: u64 = 16;
    pub const RENAME_AT: u64 = 17;
    pub const LINK_AT: u64 = 18;
}

/// Directory entry record layout produced by `Directory.list`, matching the
/// mlibc `Sysdeps<ReadEntries>` decoder: `{u64 inode, u32 type, u32
/// name_bytes, name}` packed back to back.
pub const DIRENT_RECORD_HEADER: usize = 16;
/// Entry types as served by vfsd/mlibc (1 dir, 2 symlink, else regular).
pub const DIRENT_TYPE_DIR: u32 = 1;
pub const DIRENT_TYPE_LNK: u32 = 2;

// Open mode / attribute encoding -- byte-identical to mlibc `open_path_at`.
pub const OPEN_MODE_READ: u64 = 1;
pub const OPEN_MODE_WRITE: u64 = 2;
pub const OPEN_MODE_APPEND: u64 = 8;
pub const OPEN_MODE_NONBLOCK: u64 = 16;
pub const OPEN_MODE_EXCL: u64 = 128;

pub const OPEN_ATTR_AUTO_CREATE_FILE: u64 = 1;
pub const OPEN_ATTR_DIRECTORY: u64 = 16;
pub const OPEN_ATTR_FILE: u64 = 32;
pub const OPEN_ATTR_TRUNC: u64 = 256;
pub const OPEN_ATTR_APPEND: u64 = 2048;
pub const OPEN_ATTR_EXCL: u64 = 4096;

// Seek whence codes (mlibc LSEEK_MODE_*).
pub const LSEEK_CURRENT: u64 = 0;
pub const LSEEK_BEGIN: u64 = 1;
pub const LSEEK_END: u64 = 2;

// remove() flags: bit0 selects directory removal (mlibc Unlinkat/Rmdir).
pub const REMOVE_FLAG_DIRECTORY: u64 = 1;
// create() flags: bit0 required by mlibc Mkdir/create calls.
pub const CREATE_FLAG_PRESENT: u64 = 1;

pub const MAX_PATH_BYTES: usize = 4095;
pub const MAX_MESSAGE_BYTES: usize = 65536;

unsafe extern "C" {
    fn _na_handle_close(handle: Handle) -> RawStatus;
    fn _na_memory_create(size: u64, flags: u64, result: *mut Handle) -> RawStatus;
    fn _na_memory_map(frame: *mut MemoryMapFrame) -> RawStatus;
    fn _na_memory_unmap(frame: *mut MemoryUnmapFrame) -> RawStatus;
    fn _na_handle_duplicate(source: Handle, rights: u64, result: *mut Handle) -> RawStatus;
    fn _na_handle_get_info(handle: Handle, result: *mut HandleInfo) -> RawStatus;
    fn _na_invoke_submit(
        target: Handle,
        frame: *const SubmitFrame,
        invocation: *mut Handle,
    ) -> RawStatus;
    fn _na_invocation_take_result(invocation: Handle, frame: *mut ResultFrame) -> RawStatus;
    fn _na_invocation_cancel(invocation: Handle) -> RawStatus;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SubmitFrame {
    struct_size: u32,
    flags: u32,
    method_id: u64,
    request: u64,
    request_bytes: u64,
    resources: u64,
    resource_count: u64,
    operation_budget: u64,
    reserved0: u64,
    reserved1: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResultFrame {
    struct_size: u32,
    flags: u32,
    method_id: u64,
    bytes: u64,
    byte_capacity: u64,
    resources: u64,
    resource_capacity: u64,
    actual_bytes: u64,
    actual_resources: u64,
    required_bytes: u64,
    required_resources: u64,
    execution_outcome: u32,
    outcome_reason: u32,
    protocol_error: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResourceDisposition {
    handle: Handle,
    operation: u32,
    flags: u32,
    rights: u64,
    scope: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryMapFrame {
    struct_size: u32,
    flags: u32,
    hint: u64,
    object: Handle,
    pager: Handle,
    offset: u64,
    length: u64,
    address: u64,
    data_offset: u64,
    reserved0: u64,
    reserved1: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryUnmapFrame {
    struct_size: u32,
    flags: u32,
    address: u64,
    length: u64,
    reserved0: u64,
    reserved1: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HandleInfo {
    struct_size: u32,
    binding: u32,
    scope: u64,
    revision: u64,
    features: u64,
    meta_rights: u64,
    protocol_rights: u64,
    signals: u64,
    generation: u64,
    object_state: u64,
    protocol_uuid: [u8; 16],
    object_id: u64,
    view_offset: u64,
    view_length: u64,
}

/// Failure of one protocol round trip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallError {
    /// Transport-level failure; payload is the NaOS status.
    Status(RawStatus),
    /// Typed server-side failure; payload is the negated POSIX errno from
    /// `ResultFrame.protocol_error`.
    Protocol(i64),
    Codec(&'static str),
}

impl CallError {
    /// Raw code in the shared `raw_os_error` space: statuses stay positive,
    /// protocol errnos stay negative.
    pub fn raw(self) -> i32 {
        match self {
            CallError::Status(s) => s as i32,
            CallError::Protocol(e) => e as i32,
            CallError::Codec(_) => -(status::IO_ERROR as i32),
        }
    }
}

pub fn close_handle(handle: Handle) {
    if handle != HANDLE_INVALID {
        unsafe { _na_handle_close(handle) };
    }
}

/// True when dup() of this binding must share the native handle instead of
/// calling `_na_handle_duplicate` (PRD §5.2 rule, mirrored from mlibc).
pub fn binding_is_sharable(handle: Handle) -> bool {
    let mut info = HandleInfo {
        struct_size: core::mem::size_of::<HandleInfo>() as u32,
        ..HandleInfo::default()
    };
    if unsafe { _na_handle_get_info(handle, &mut info) } != status::OK {
        return false;
    }
    info.scope == SCOPE_FILE || info.scope == SCOPE_DIRECTORY
}

pub fn duplicate_stream_handle(source: Handle) -> Result<Handle, CallError> {
    let mut duplicated = HANDLE_INVALID;
    let st = unsafe { _na_handle_duplicate(source, 0, &mut duplicated) };
    if st != status::OK {
        return Err(CallError::Status(st));
    }
    Ok(duplicated)
}

/// Minimal canonical encoder (little-endian, no padding).
struct Encoder<'a> {
    buffer: &'a mut [u8],
    offset: usize,
}

impl<'a> Encoder<'a> {
    fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    fn put_u32(&mut self, value: u32) -> Result<(), CallError> {
        self.put_raw(&value.to_le_bytes())
    }

    fn put_u64(&mut self, value: u64) -> Result<(), CallError> {
        self.put_raw(&value.to_le_bytes())
    }

    fn put_i64(&mut self, value: i64) -> Result<(), CallError> {
        self.put_u64(value as u64)
    }

    fn put_raw(&mut self, value: &[u8]) -> Result<(), CallError> {
        let end = self.offset.checked_add(value.len()).ok_or(CallError::Codec("overflow"))?;
        if end > self.buffer.len() {
            return Err(CallError::Codec("overflow"));
        }
        self.buffer[self.offset..end].copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn written(&self) -> usize {
        self.offset
    }
}

/// Minimal canonical decoder over a complete response message.
struct Decoder<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    fn get_u32(&mut self) -> Result<u32, CallError> {
        let raw = self.get_raw(4)?;
        Ok(u32::from_le_bytes(raw.try_into().unwrap()))
    }

    fn get_u64(&mut self) -> Result<u64, CallError> {
        let raw = self.get_raw(8)?;
        Ok(u64::from_le_bytes(raw.try_into().unwrap()))
    }

    fn get_i64(&mut self) -> Result<i64, CallError> {
        Ok(self.get_u64()? as i64)
    }

    fn rest(&self) -> &'a [u8] {
        &self.buffer[self.offset..]
    }

    fn get_raw(&mut self, size: usize) -> Result<&'a [u8], CallError> {
        let end = self.offset.checked_add(size).ok_or(CallError::Codec("truncated"))?;
        if end > self.buffer.len() {
            return Err(CallError::Codec("truncated"));
        }
        let value = &self.buffer[self.offset..end];
        self.offset = end;
        Ok(value)
    }
}

const MAX_RESOURCES: usize = 64;

/// Outcome of a completed call: decoded value plus any moved-in response
/// handle the caller asked to claim via `claim_slot`.
struct Completion<T> {
    value: T,
    /// The handle at `claim_slot`, if one was requested.
    claimed: Option<Handle>,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EpollEvent {
    events: u64,
    data: u64,
}

const EPOLL_EVENT_READABLE: u64 = 1 << 0;
const EPOLL_EVENT_HANGUP: u64 = 1 << 3;
const EPOLL_CTL_ADD: u32 = 1;

unsafe extern "C" {
    fn _na_epoll_create(result: *mut Handle) -> RawStatus;
    fn _na_epoll_ctl(epoll: Handle, operation: u32, target: Handle, event: *const EpollEvent) -> RawStatus;
    fn _na_epoll_wait(
        epoll: Handle,
        events: *mut EpollEvent,
        capacity: u64,
        actual: *mut u64,
        deadline: *const u8,
    ) -> RawStatus;
}

fn wait_invocation(invocation: Handle) -> RawStatus {
    let mut epoll = HANDLE_INVALID;
    let st = unsafe { _na_epoll_create(&mut epoll) };
    if st != status::OK {
        return st;
    }
    let event = EpollEvent {
        events: EPOLL_EVENT_READABLE | EPOLL_EVENT_HANGUP,
        data: 0,
    };
    let st = unsafe { _na_epoll_ctl(epoll, EPOLL_CTL_ADD, invocation, &event) };
    if st != status::OK {
        unsafe { _na_handle_close(epoll) };
        return st;
    }
    let mut returned = [EpollEvent::default(); 1];
    let mut actual = 0;
    let st = unsafe {
        _na_epoll_wait(
            epoll,
            returned.as_mut_ptr(),
            1,
            &mut actual,
            core::ptr::null(),
        )
    };
    unsafe { _na_handle_close(epoll) };
    if st != status::OK {
        return st;
    }
    if actual != 0 && returned[0].events & EPOLL_EVENT_READABLE != 0 {
        status::OK
    } else if actual != 0 && returned[0].events & EPOLL_EVENT_HANGUP != 0 {
        status::PEER_CLOSED
    } else {
        // A successful wait must observe one of the requested terminal
        // signals. Do not fall through to take_result and turn this into a
        // racy WOULD_BLOCK round trip.
        status::WOULD_BLOCK
    }
}

/// Wait for and consume an invocation while keeping ownership explicit.
/// Every post-submit path closes the invocation exactly once; paths that do
/// not successfully take a result also request cancellation first.
fn await_and_take(
    invocation: Handle,
    result: &mut ResultFrame,
    wait: impl FnOnce(Handle) -> RawStatus,
    take: impl FnOnce(Handle, *mut ResultFrame) -> RawStatus,
    cancel: impl FnOnce(Handle) -> RawStatus,
    close: impl FnOnce(Handle) -> RawStatus,
) -> Result<(), CallError> {
    let wait_status = wait(invocation);
    if wait_status != status::OK {
        let _ = cancel(invocation);
        let _ = close(invocation);
        return Err(CallError::Status(wait_status));
    }

    let take_status = take(invocation, result as *mut ResultFrame);
    if take_status != status::OK {
        let _ = cancel(invocation);
        let _ = close(invocation);
        return Err(CallError::Status(take_status));
    }

    let _ = close(invocation);
    Ok(())
}

fn invoke<T>(
    target: Handle,
    method: u64,
    encode: impl FnOnce(&mut Encoder) -> Result<(), CallError>,
    decode: impl FnOnce(&mut Decoder) -> Result<T, CallError>,
    resources: Option<&[ResourceDisposition]>,
    // Response resource slot whose moved handle the caller wants to own;
    // every other received handle is closed immediately.
    claim_slot: Option<u32>,
) -> Result<Completion<T>, CallError> {
    let mut wire = [0u8; MAX_MESSAGE_BYTES];
    let mut encoder = Encoder::new(&mut wire);
    encode(&mut encoder)?;
    let request_bytes = encoder.written();

    let (resources_ptr, resource_count) = match resources {
        Some(list) => (list.as_ptr() as u64, list.len() as u64),
        None => (0, 0),
    };
    let frame = SubmitFrame {
        struct_size: core::mem::size_of::<SubmitFrame>() as u32,
        method_id: method,
        request: if request_bytes == 0 { 0 } else { wire.as_ptr() as u64 },
        request_bytes: request_bytes as u64,
        resources: resources_ptr,
        resource_count,
        ..SubmitFrame::default()
    };
    let mut invocation = HANDLE_INVALID;
    let st = unsafe { _na_invoke_submit(target, &frame, &mut invocation) };
    if st != status::OK {
        return Err(CallError::Status(st));
    }
    debug_assert!(invocation != HANDLE_INVALID);

    let mut moved_handles = [HANDLE_INVALID; MAX_RESOURCES];
    let mut result = ResultFrame {
        struct_size: core::mem::size_of::<ResultFrame>() as u32,
        bytes: wire.as_mut_ptr() as u64,
        byte_capacity: wire.len() as u64,
        resources: moved_handles.as_mut_ptr() as u64,
        resource_capacity: MAX_RESOURCES as u64,
        ..ResultFrame::default()
    };
    await_and_take(
        invocation,
        &mut result,
        wait_invocation,
        |handle, frame| unsafe { _na_invocation_take_result(handle, frame) },
        |handle| unsafe { _na_invocation_cancel(handle) },
        |handle| unsafe { _na_handle_close(handle) },
    )?;
    if result.method_id != method
        || result.actual_bytes > wire.len() as u64
        || result.actual_resources > MAX_RESOURCES as u64
    {
        return Err(CallError::Codec("malformed result frame"));
    }
    if result.execution_outcome != 0 || result.protocol_error != 0 {
        return Err(CallError::Protocol(result.protocol_error));
    }

    // The kernel installs received handles directly into the resource array
    // supplied via `ResultFrame.resources`. The first `actual_resources`
    // entries are valid; anything the caller did not claim is closed here.
    let moved_count = result.actual_resources as usize;
    let mut claimed = None;
    for slot in 0..moved_count as u32 {
        if claim_slot == Some(slot) {
            claimed = Some(moved_handles[slot as usize]);
        } else {
            close_handle(moved_handles[slot as usize]);
            moved_handles[slot as usize] = HANDLE_INVALID;
        }
    }

    let mut decoder = Decoder::new(&wire[..result.actual_bytes as usize]);
    let value = decode(&mut decoder)?;
    Ok(Completion { value, claimed })
}

#[cfg(test)]
mod invocation_lifecycle_tests {
    use core::cell::Cell;

    use super::{CallError, ResultFrame, await_and_take};
    use crate::sys::pal::naos::errors::status;

    fn result_frame() -> ResultFrame {
        ResultFrame {
            struct_size: core::mem::size_of::<ResultFrame>() as u32,
            ..ResultFrame::default()
        }
    }

    #[test]
    fn waits_before_taking_an_async_result() {
        let waited = Cell::new(false);
        let took_after_wait = Cell::new(false);
        let mut result = result_frame();
        let completion = await_and_take(
            17,
            &mut result,
            |invocation| {
                assert_eq!(invocation, 17);
                waited.set(true);
                status::OK
            },
            |_invocation, _frame| {
                took_after_wait.set(waited.get());
                status::OK
            },
            |_invocation| panic!("a completed invocation must not be cancelled"),
            |_invocation| status::OK,
        );

        assert!(completion.is_ok());
        assert!(took_after_wait.get());
    }

    #[test]
    fn closes_after_success_without_cancelling() {
        let cancelled = Cell::new(0);
        let closed = Cell::new(0);
        let mut result = result_frame();
        let completion = await_and_take(
            17,
            &mut result,
            |_invocation| status::OK,
            |_invocation, _frame| status::OK,
            |_invocation| {
                cancelled.set(cancelled.get() + 1);
                status::OK
            },
            |_invocation| {
                closed.set(closed.get() + 1);
                status::OK
            },
        );

        assert!(completion.is_ok());
        assert_eq!(cancelled.get(), 0);
        assert_eq!(closed.get(), 1);
    }

    #[test]
    fn peer_close_cancels_and_closes_without_taking() {
        let taken = Cell::new(0);
        let cancelled = Cell::new(0);
        let closed = Cell::new(0);
        let mut result = result_frame();
        let completion = await_and_take(
            17,
            &mut result,
            |_invocation| status::PEER_CLOSED,
            |_invocation, _frame| {
                taken.set(taken.get() + 1);
                status::OK
            },
            |_invocation| {
                cancelled.set(cancelled.get() + 1);
                status::OK
            },
            |_invocation| {
                closed.set(closed.get() + 1);
                status::OK
            },
        );

        assert_eq!(completion, Err(CallError::Status(status::PEER_CLOSED)));
        assert_eq!(taken.get(), 0);
        assert_eq!(cancelled.get(), 1);
        assert_eq!(closed.get(), 1);
    }

    #[test]
    fn wait_error_cancels_and_closes() {
        let cancelled = Cell::new(0);
        let closed = Cell::new(0);
        let mut result = result_frame();
        let completion = await_and_take(
            17,
            &mut result,
            |_invocation| status::IO_ERROR,
            |_invocation, _frame| panic!("take must not run after wait failure"),
            |_invocation| {
                cancelled.set(cancelled.get() + 1);
                status::OK
            },
            |_invocation| {
                closed.set(closed.get() + 1);
                status::OK
            },
        );

        assert_eq!(completion, Err(CallError::Status(status::IO_ERROR)));
        assert_eq!(cancelled.get(), 1);
        assert_eq!(closed.get(), 1);
    }

    #[test]
    fn take_error_cancels_and_closes() {
        let cancelled = Cell::new(0);
        let closed = Cell::new(0);
        let mut result = result_frame();
        let completion = await_and_take(
            17,
            &mut result,
            |_invocation| status::OK,
            |_invocation, _frame| status::INVALID_MESSAGE,
            |_invocation| {
                cancelled.set(cancelled.get() + 1);
                status::OK
            },
            |_invocation| {
                closed.set(closed.get() + 1);
                status::OK
            },
        );

        assert_eq!(completion, Err(CallError::Status(status::INVALID_MESSAGE)));
        assert_eq!(cancelled.get(), 1);
        assert_eq!(closed.get(), 1);
    }
}

/// POSIX-style stat payload (`Directory.Stat` / `File.Stat`, 20 fixed fields
/// in @id order).
#[derive(Clone, Copy, Debug, Default)]
pub struct Stat {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub padding: u32,
    pub device_id: u64,
    pub size: i64,
    pub block_size: i64,
    pub blocks: i64,
    pub access_seconds: i64,
    pub access_nanoseconds: i64,
    pub modify_seconds: i64,
    pub modify_nanoseconds: i64,
    pub change_seconds: i64,
    pub change_nanoseconds: i64,
    pub unused: [i64; 3],
}

fn decode_stat(decoder: &mut Decoder) -> Result<Stat, CallError> {
    let mut stat = Stat::default();
    stat.device = decoder.get_u64()?;
    stat.inode = decoder.get_u64()?;
    stat.links = decoder.get_u64()?;
    stat.mode = decoder.get_u32()?;
    stat.uid = decoder.get_u32()?;
    stat.gid = decoder.get_u32()?;
    stat.padding = decoder.get_u32()?;
    stat.device_id = decoder.get_u64()?;
    stat.size = decoder.get_i64()?;
    stat.block_size = decoder.get_i64()?;
    stat.blocks = decoder.get_i64()?;
    stat.access_seconds = decoder.get_i64()?;
    stat.access_nanoseconds = decoder.get_i64()?;
    stat.modify_seconds = decoder.get_i64()?;
    stat.modify_nanoseconds = decoder.get_i64()?;
    stat.change_seconds = decoder.get_i64()?;
    stat.change_nanoseconds = decoder.get_i64()?;
    stat.unused = [decoder.get_i64()?, decoder.get_i64()?, decoder.get_i64()?];
    Ok(stat)
}

struct BulkRegion {
    handle: Handle,
    address: *mut u8,
    length: usize,
}

impl BulkRegion {
    fn new(length: usize) -> Result<Self, CallError> {
        let length = length.max(1).checked_add(4095).ok_or(CallError::Codec("size"))? & !4095;
        let mut handle = HANDLE_INVALID;
        let status = unsafe { _na_memory_create(length as u64, 0, &mut handle) };
        if status != status::OK || handle == HANDLE_INVALID {
            return Err(CallError::Status(status));
        }
        let mut frame = MemoryMapFrame {
            struct_size: core::mem::size_of::<MemoryMapFrame>() as u32,
            flags: MEMORY_MAP_READ | MEMORY_MAP_WRITE | MEMORY_MAP_SHARED,
            object: handle,
            length: length as u64,
            ..MemoryMapFrame::default()
        };
        let status = unsafe { _na_memory_map(&mut frame) };
        if status != status::OK || frame.address == 0 {
            unsafe { _na_handle_close(handle) };
            return Err(CallError::Status(status));
        }
        Ok(Self { handle, address: frame.address as *mut u8, length })
    }

    fn resource(&self) -> ResourceDisposition {
        ResourceDisposition {
            handle: self.handle,
            operation: RESOURCE_DUPLICATE,
            ..ResourceDisposition::default()
        }
    }

    fn write(&self, data: &[u8]) -> Result<(), CallError> {
        if data.len() > self.length {
            return Err(CallError::Codec("bulk overflow"));
        }
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), self.address, data.len()) };
        Ok(())
    }

    fn read(&self, data: &mut [u8]) -> Result<(), CallError> {
        if data.len() > self.length {
            return Err(CallError::Codec("bulk overflow"));
        }
        unsafe { core::ptr::copy_nonoverlapping(self.address, data.as_mut_ptr(), data.len()) };
        Ok(())
    }
}

impl Drop for BulkRegion {
    fn drop(&mut self) {
        let mut frame = MemoryUnmapFrame {
            struct_size: core::mem::size_of::<MemoryUnmapFrame>() as u32,
            address: self.address as u64,
            length: self.length as u64,
            ..MemoryUnmapFrame::default()
        };
        unsafe {
            let _ = _na_memory_unmap(&mut frame);
            let _ = _na_handle_close(self.handle);
        }
    }
}

// ---------------------------------------------------------------------------
// File operations (revision 6)
// ---------------------------------------------------------------------------

pub fn file_read(target: Handle, size: u64, buf: &mut [u8]) -> Result<usize, CallError> {
    let size = usize::try_from(size).map_err(|_| CallError::Codec("size"))?;
    if size > buf.len() {
        return Err(CallError::Codec("read overflow"));
    }
    if size == 0 {
        return Ok(0);
    }
    let region = BulkRegion::new(size)?;
    let resource = region.resource();
    let completion = invoke(
        target,
        file_method::READ,
        |enc| {
            enc.put_u64(size as u64)?; // size @id(1)
            enc.put_u64(0)?; // flags @id(2)
            enc.put_u32(0)?; // buffer @id(3)
            Ok(())
        },
        |dec| dec.get_u64(),
        Some(core::slice::from_ref(&resource)),
        None,
    )?;
    let count = usize::try_from(completion.value).map_err(|_| CallError::Codec("read count"))?;
    if count > size {
        return Err(CallError::Codec("read count"));
    }
    region.read(&mut buf[..count])?;
    Ok(count)
}

pub fn file_write(target: Handle, data: &[u8]) -> Result<u64, CallError> {
    if data.is_empty() {
        return Ok(0);
    }
    let region = BulkRegion::new(data.len())?;
    region.write(data)?;
    let resource = region.resource();
    let completion = invoke(
        target,
        file_method::WRITE,
        |enc| {
            enc.put_u64(data.len() as u64)?; // size @id(1)
            enc.put_u64(0)?; // flags @id(2)
            enc.put_u32(0)?; // buffer @id(3)
            Ok(())
        },
        |dec| dec.get_u64(),
        Some(core::slice::from_ref(&resource)),
        None,
    )?;
    Ok(completion.value)
}

pub fn file_seek(target: Handle, offset: i64, whence: u64) -> Result<i64, CallError> {
    let completion = invoke(
        target,
        file_method::SEEK,
        |enc| {
            enc.put_i64(offset)?; // offset @id(1)
            enc.put_u64(whence)?; // whence @id(2)
            Ok(())
        },
        |dec| dec.get_i64(),
        None,
        None,
    )?;
    Ok(completion.value)
}

pub fn file_stat(target: Handle) -> Result<Stat, CallError> {
    let completion = invoke(target, file_method::STAT, |_| Ok(()), decode_stat, None, None)?;
    Ok(completion.value)
}

pub fn file_sync(target: Handle) -> Result<(), CallError> {
    invoke(target, file_method::SYNC, |_| Ok(()), |_| Ok(()), None, None)?;
    Ok(())
}

pub fn file_truncate(target: Handle, length: u64) -> Result<(), CallError> {
    invoke(
        target,
        file_method::TRUNCATE,
        |enc| {
            enc.put_u64(length)?;
            Ok(())
        },
        |_| Ok(()),
        None,
        None,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Directory operations (revision 2)
// ---------------------------------------------------------------------------

/// `Directory.open`. `path` is sent NUL-terminated exactly like mlibc
/// (`bytes<4096>` without an explicit length field). Returns the moved
/// object handle from the response.
pub fn dir_open(
    target: Handle,
    mode: u64,
    attributes: u64,
    path: &[u8],
) -> Result<Handle, CallError> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES + 1 || *path.last().unwrap() != 0 {
        return Err(CallError::Codec("path"));
    }
    let completion = invoke(
        target,
        dir_method::OPEN,
        |enc| {
            enc.put_u64(mode)?; // mode @id(1)
            enc.put_u64(attributes)?; // flags @id(2)
            enc.put_raw(path)?; // path @id(3), NUL included
            Ok(())
        },
        |dec| dec.get_u32(),
        None,
        Some(0), // object @id(1): the single moved response handle
    )?;
    completion.claimed.ok_or(CallError::Codec("missing open handle"))
}

/// One decoded `Directory.list` record.
pub struct DirEntryRecord {
    pub inode: u64,
    pub kind: u32,
    pub name: [u8; 255],
    pub name_len: usize,
}

/// `Directory.list` cursor step. Returns `(next_offset, entries)`; an empty
/// vector means end-of-directory.
pub fn dir_list(target: Handle, offset: u64) -> Result<(u64, Vec<DirEntryRecord>), CallError> {
    let completion = invoke(
        target,
        dir_method::LIST,
        |enc| {
            enc.put_u64(offset)?; // offset @id(1)
            enc.put_u64(MAX_MESSAGE_BYTES as u64)?; // requested_bytes @id(2)
            Ok(())
        },
        |dec| {
            let next = dec.get_u64()?; // next @id(1)
            let count = dec.get_u64()?; // count @id(2)
            let records = dec.rest(); // records @id(3), trailing blob
            let mut entries = Vec::new();
            let mut cursor = records;
            for _ in 0..count {
                if cursor.len() < DIRENT_RECORD_HEADER {
                    return Err(CallError::Codec("dirent header"));
                }
                let inode = u64::from_le_bytes(cursor[0..8].try_into().unwrap());
                let kind = u32::from_le_bytes(cursor[8..12].try_into().unwrap());
                let record_name_len =
                    u32::from_le_bytes(cursor[12..16].try_into().unwrap()) as usize;
                // `name_bytes` includes the C-compatible trailing NUL in the
                // shared Directory.list wire contract. Keep that byte in the
                // record stride, but never expose it as part of OsString.
                if record_name_len < 2
                    || record_name_len > 256
                    || DIRENT_RECORD_HEADER + record_name_len > cursor.len()
                {
                    return Err(CallError::Codec("dirent name"));
                }
                let encoded_name = &cursor[16..16 + record_name_len];
                if encoded_name[record_name_len - 1] != 0 {
                    return Err(CallError::Codec("dirent terminator"));
                }
                let name_len = record_name_len - 1;
                let mut name = [0u8; 255];
                name[..name_len].copy_from_slice(&encoded_name[..name_len]);
                entries.push(DirEntryRecord { inode, kind, name, name_len });
                cursor = &cursor[DIRENT_RECORD_HEADER + record_name_len..];
            }
            Ok((next, entries))
        },
        None,
        None,
    )?;
    Ok(completion.value)
}

pub fn dir_stat(target: Handle) -> Result<Stat, CallError> {
    let completion = invoke(target, dir_method::STAT, |_| Ok(()), decode_stat, None, None)?;
    Ok(completion.value)
}

fn check_path_nul(path: &[u8]) -> Result<(), CallError> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES + 1 || *path.last().unwrap() != 0 {
        return Err(CallError::Codec("path"));
    }
    Ok(())
}

/// `Directory.create` (mkdir or create-file variant chosen by caller flags).
pub fn dir_create(target: Handle, mode: u64, flags: u64, path: &[u8]) -> Result<(), CallError> {
    check_path_nul(path)?;
    invoke(
        target,
        dir_method::CREATE,
        |enc| {
            enc.put_u64(mode)?;
            enc.put_u64(flags)?;
            enc.put_raw(path)?;
            Ok(())
        },
        |_| Ok(()),
        None,
        None,
    )?;
    Ok(())
}

/// `Directory.remove`; `flags.bit0` selects rmdir semantics.
pub fn dir_remove(target: Handle, mode: u64, flags: u64, path: &[u8]) -> Result<(), CallError> {
    check_path_nul(path)?;
    invoke(
        target,
        dir_method::REMOVE,
        |enc| {
            enc.put_u64(mode)?;
            enc.put_u64(flags)?;
            enc.put_raw(path)?;
            Ok(())
        },
        |_| Ok(()),
        None,
        None,
    )?;
    Ok(())
}

/// `Directory.path` -- absolute path of the bound directory.
pub fn dir_path(target: Handle) -> Result<Vec<u8>, CallError> {
    let completion = invoke(
        target,
        dir_method::PATH,
        |_| Ok(()),
        |dec| {
            let blob = dec.rest();
            let trimmed = match blob.iter().rposition(|&b| b == 0) {
                Some(pos) => &blob[..pos], // strip trailing NUL when present
                None => blob,
            };
            Ok(Vec::from(trimmed))
        },
        None,
        None,
    )?;
    Ok(completion.value)
}

/// Path-form query replacing open+stat (`stat_node`); NOFOLLOW applies to the
/// final component only. `path` has NO trailing NUL (explicit length field).
pub fn dir_stat_node(target: Handle, nofollow: bool, path: &[u8]) -> Result<Stat, CallError> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES {
        return Err(CallError::Codec("path"));
    }
    let flags = if nofollow { NA_DIRECTORY_LOOKUP_FLAG_NOFOLLOW } else { 0 };
    let completion = invoke(
        target,
        dir_method::STAT_NODE,
        |enc| {
            enc.put_u64(flags)?; // flags @id(1)
            enc.put_u64(path.len() as u64)?; // path_size @id(2)
            enc.put_raw(path)?; // path @id(3)
            Ok(())
        },
        decode_stat,
        None,
        None,
    )?;
    Ok(completion.value)
}

/// `Directory.clone_binding` -- a fresh unique endpoint for fork/spawn or
/// MOVE-disposition requests. Never used for same-process dup().
pub fn dir_clone_binding(target: Handle) -> Result<Handle, CallError> {
    let completion = invoke(
        target,
        dir_method::CLONE_BINDING,
        |_| Ok(()),
        |dec| dec.get_u32(),
        None,
        Some(0), // directory @id(1): the single moved response handle
    )?;
    completion.claimed.ok_or(CallError::Codec("missing cloned binding"))
}

fn check_path_raw(first: &[u8], second: &[u8]) -> Result<(), CallError> {
    if first.is_empty()
        || first.len() > MAX_PATH_BYTES
        || second.is_empty()
        || second.len() > MAX_PATH_BYTES
    {
        return Err(CallError::Codec("path"));
    }
    Ok(())
}

/// `Directory.symlink` (first = target, second = link path).
pub fn dir_symlink(target: Handle, first: &[u8], second: &[u8]) -> Result<(), CallError> {
    check_path_raw(first, second)?;
    invoke(
        target,
        dir_method::SYMLINK,
        |enc| {
            enc.put_u64(first.len() as u64)?; // first_size @id(1)
            enc.put_u64(second.len() as u64)?; // second_size @id(2)
            enc.put_raw(first)?; // first @id(3)
            enc.put_raw(second)?; // second @id(4)
            Ok(())
        },
        |_| Ok(()),
        None,
        None,
    )?;
    Ok(())
}

/// `Directory.readlink`; returns the link target bytes.
pub fn dir_readlink(target: Handle, path: &[u8]) -> Result<Vec<u8>, CallError> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES {
        return Err(CallError::Codec("path"));
    }
    let completion = invoke(
        target,
        dir_method::READLINK,
        |enc| {
            enc.put_u64(path.len() as u64)?; // size @id(1)
            enc.put_raw(path)?; // path @id(2)
            Ok(())
        },
        |dec| {
            let blob = dec.rest(); // target @id(1), trailing blob
            let trimmed = match blob.iter().rposition(|&b| b == 0) {
                Some(pos) => &blob[..pos],
                None => blob,
            };
            Ok(Vec::from(trimmed))
        },
        None,
        None,
    )?;
    Ok(completion.value)
}

/// Shared implementation of `rename_at` / `link_at`: clones the new-parent
/// binding and MOVEs the temporary copy into the request, keeping the caller's
/// original handle untouched. On any failure the unconsumed clone is closed
/// best-effort (mirrors mlibc `directory_pair_at_call`).
fn pair_at_call(
    old_target: Handle,
    new_target: Handle,
    method: u64,
    flags: u64,
    first: &[u8],
    second: &[u8],
) -> Result<(), CallError> {
    check_path_raw(first, second)?;
    let clone = dir_clone_binding(new_target)?;

    let disposition = ResourceDisposition {
        handle: clone,
        operation: RESOURCE_MOVE,
        ..ResourceDisposition::default()
    };
    let result = invoke(
        old_target,
        method,
        |enc| {
            enc.put_u64(flags)?; // flags @id(1)
            enc.put_u32(0)?; // new_parent @id(2): resource slot 0
            enc.put_u64(first.len() as u64)?; // first_size @id(3)
            enc.put_u64(second.len() as u64)?; // second_size @id(4)
            enc.put_raw(first)?; // first @id(5)
            enc.put_raw(second)?; // second @id(6)
            Ok(())
        },
        |_| Ok(()),
        Some(core::slice::from_ref(&disposition)),
        None,
    );
    if result.is_err() {
        // Best effort: the kernel either restored or discarded the moved
        // clone; closing an already-consumed handle is a no-op.
        close_handle(clone);
    }
    result.map(|_| ())
}

/// `fs::rename` maps here (PRD §5.3 item 6): two-dirfd atomic rename with a
/// MOVE'd clone_binding temporary as the new parent.
pub fn dir_rename_at(
    old_target: Handle,
    new_target: Handle,
    first: &[u8],
    second: &[u8],
) -> Result<(), CallError> {
    pair_at_call(old_target, new_target, dir_method::RENAME_AT, 0, first, second)
}

/// `fs::hard_link` maps here (PRD §5.3 item 6); flags bit0 would carry
/// AT_SYMLINK_FOLLOW semantics, which std never sets.
pub fn dir_link_at(
    old_target: Handle,
    new_target: Handle,
    first: &[u8],
    second: &[u8],
) -> Result<(), CallError> {
    pair_at_call(old_target, new_target, dir_method::LINK_AT, 0, first, second)
}

/// `Directory.sync` -- fsync(dirfd) for directory bindings.
pub fn dir_sync(target: Handle) -> Result<(), CallError> {
    invoke(target, dir_method::SYNC, |_| Ok(()), |_| Ok(()), None, None)?;
    Ok(())
}
