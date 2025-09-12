use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use spin::{Lazy, Mutex};
use x86_64::instructions::interrupts;

pub const HEAP_START_ADDR: usize = 0x400_0000;
pub const HEAP_SAFE_MARGIN: usize = 0x1000;
pub const HEAP_SIZE: usize = 0x200_0000;

pub struct MemoryAllocator {
    arena: *mut u8,
    used: Lazy<Mutex<usize>>,
    mem_error_notified: Lazy<Mutex<bool>>,
}

#[global_allocator]
static MEM_ALLOC: MemoryAllocator = MemoryAllocator {
    arena: HEAP_START_ADDR as *mut u8,
    used: Lazy::new(|| Mutex::new(0)),
    mem_error_notified: Lazy::new(|| Mutex::new(false)),
};

unsafe impl Sync for MemoryAllocator {}

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Interrupt during memory allocation can corrupt the memory.
        // Also, it can cause a deadlock due to the mutexes.
        let int_enabled = interrupts::are_enabled();
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

        if int_enabled {
            interrupts::enable();
        }

        allocated
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing.
    }
}

impl MemoryAllocator {
    pub fn check_mem_error() -> bool {
        let mut mem_error_notified = MEM_ALLOC.mem_error_notified.lock();
        if *mem_error_notified {
            false
        } else {
            let used = MEM_ALLOC.used.lock();
            if *used >= HEAP_SIZE - HEAP_SAFE_MARGIN {
                *mem_error_notified = true;
                true
            } else {
                false
            }
        }
    }
}
