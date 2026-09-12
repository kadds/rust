use crate::alloc::{GlobalAlloc, Layout, System};

unsafe extern "C" {
    fn __naos_runtime_alloc(size: usize, align: usize) -> *mut u8;
    fn __naos_runtime_alloc_zeroed(size: usize, align: usize) -> *mut u8;
    fn __naos_runtime_dealloc(pointer: *mut u8, size: usize, align: usize);
    fn __naos_runtime_realloc(
        pointer: *mut u8,
        old_size: usize,
        align: usize,
        new_size: usize,
    ) -> *mut u8;
}

#[stable(feature = "alloc_system_type", since = "1.28.0")]
unsafe impl GlobalAlloc for System {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { __naos_runtime_alloc(layout.size(), layout.align()) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { __naos_runtime_alloc_zeroed(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { __naos_runtime_dealloc(pointer, layout.size(), layout.align()) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { __naos_runtime_realloc(pointer, layout.size(), layout.align(), new_size) }
    }
}
