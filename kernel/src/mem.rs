use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use spin::{Lazy, Mutex};
use x86_64::instructions::interrupts;

pub const HEAP_START_ADDR: usize = 0x4_000_000;
pub const HEAP_SIZE: usize = 0x2_000_000;

struct MemoryAllocator {
    arena: *mut u8,
    used: Lazy<Mutex<usize>>,
}

#[global_allocator]
static MEM_ALLOC: MemoryAllocator = MemoryAllocator {
    arena: HEAP_START_ADDR as *mut u8,
    used: Lazy::new(|| Mutex::new(0)),
};

unsafe impl Sync for MemoryAllocator {}

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Interrupt during memory allocation can corrupt the memory.
        // Also, it can cause a deadlock due to the mutexes.
        interrupts::disable();

        let allocated = {
            let mut used = self.used.lock();

            let size = layout.size();
            let align = layout.align();

            let start_offset = ((*used + align - 1) / align) * align;
            let end_offset = start_offset + size;
            if end_offset >= HEAP_SIZE {
                return null_mut();
            }
            *used = end_offset;

            unsafe { self.arena.add(start_offset) }
        };

        interrupts::enable();

        allocated
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing.
    }
}
