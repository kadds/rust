unsafe extern "C" {
    fn __naos_runtime_getrandom(buffer: *mut u8, length: usize) -> i32;
}

pub fn fill_bytes(bytes: &mut [u8]) {
    if bytes.is_empty() {
        return;
    }

    if unsafe { __naos_runtime_getrandom(bytes.as_mut_ptr(), bytes.len()) } != 0 {
        crate::sys::pal::abort_internal()
    }
}

pub fn hashmap_random_keys() -> (u64, u64) {
    let mut bytes = [0_u8; 16];
    fill_bytes(&mut bytes);
    let first = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
    let second = u64::from_ne_bytes(bytes[8..].try_into().unwrap());
    (first, second)
}
