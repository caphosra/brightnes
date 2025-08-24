use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

pub const HEAP_SIZE: usize = 0x400_000;

#[repr(C, align(4096))]
pub struct MemoryAllocator {
    arena: [u8; HEAP_SIZE],
    used: usize,
}

#[global_allocator]
static mut MEM_ALLOC: MemoryAllocator = MemoryAllocator {
    arena: [0; HEAP_SIZE],
    used: 0,
};

unsafe impl Sync for MemoryAllocator {}

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        let start_offset = ((self.used + align - 1) / align) * align;
        let end_offset = start_offset + size;
        if end_offset >= HEAP_SIZE {
            return null_mut();
        }

        unsafe {
            MEM_ALLOC.used = end_offset;
        }

        unsafe { MEM_ALLOC.arena.as_mut_ptr().add(start_offset) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing.
    }
}
