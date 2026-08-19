pub fn fill_bytes(_: &mut [u8]) {
    crate::sys::pal::abort_internal()
}

pub fn hashmap_random_keys() -> (u64, u64) {
    crate::sys::pal::abort_internal()
}
