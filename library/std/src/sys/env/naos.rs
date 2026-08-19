use crate::ffi::{OsStr, OsString};
use crate::{fmt, io, sync::Mutex, sync::OnceLock, vec};

#[repr(C)]
#[derive(Clone, Copy)]
struct SnapshotSlice {
    pointer: *const u8,
    length: usize,
}

unsafe extern "C" {
    fn __naos_runtime_env_count() -> usize;
    fn __naos_runtime_env(index: usize) -> SnapshotSlice;
}

type Environment = vec::Vec<(OsString, OsString)>;
static ENVIRONMENT: OnceLock<Mutex<Environment>> = OnceLock::new();

fn encoded(slice: SnapshotSlice) -> Option<OsString> {
    if slice.pointer.is_null() {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(slice.pointer, slice.length) }.to_vec();
    Some(unsafe { OsString::from_encoded_bytes_unchecked(bytes) })
}

fn environment() -> &'static Mutex<Environment> {
    ENVIRONMENT.get_or_init(|| {
        let count = unsafe { __naos_runtime_env_count() };
        let mut values = Environment::new();
        for index in 0..count {
            let Some(value) = encoded(unsafe { __naos_runtime_env(index) }) else {
                continue;
            };
            let Some(separator) = value.as_encoded_bytes().iter().position(|byte| *byte == b'=') else {
                continue;
            };
            let bytes = value.into_encoded_bytes();
            let key = unsafe { OsString::from_encoded_bytes_unchecked(bytes[..separator].to_vec()) };
            let value = unsafe { OsString::from_encoded_bytes_unchecked(bytes[separator + 1..].to_vec()) };
            if let Some((_, old_value)) = values.iter_mut().find(|(old_key, _)| *old_key == key) {
                *old_value = value;
            } else {
                values.push((key, value));
            }
        }
        Mutex::new(values)
    })
}

pub struct Env {
    iter: vec::IntoIter<(OsString, OsString)>,
}

impl fmt::Debug for Env {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter.as_slice()).finish()
    }
}

impl Iterator for Env {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub fn env() -> Env {
    let values = environment().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
    Env { iter: values.into_iter() }
}

pub fn getenv(key: &OsStr) -> Option<OsString> {
    environment()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .find(|(old_key, _)| old_key == key)
        .map(|(_, value)| value.clone())
}

fn validate(key: &OsStr, value: Option<&OsStr>) -> io::Result<()> {
    if key.as_encoded_bytes().contains(&0) || key.as_encoded_bytes().contains(&b'=') {
        return Err(io::const_error!(io::ErrorKind::InvalidInput, "invalid environment key"));
    }
    if value.is_some_and(|value| value.as_encoded_bytes().contains(&0)) {
        return Err(io::const_error!(io::ErrorKind::InvalidInput, "invalid environment value"));
    }
    Ok(())
}

pub unsafe fn setenv(key: &OsStr, value: &OsStr) -> io::Result<()> {
    validate(key, Some(value))?;
    let mut environment = environment().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, old_value)) = environment.iter_mut().find(|(old_key, _)| old_key == key) {
        *old_value = value.to_os_string();
    } else {
        environment.push((key.to_os_string(), value.to_os_string()));
    }
    Ok(())
}

pub unsafe fn unsetenv(key: &OsStr) -> io::Result<()> {
    validate(key, None)?;
    let mut environment = environment().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    environment.retain(|(old_key, _)| old_key != key);
    Ok(())
}
