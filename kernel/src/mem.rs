use core::alloc::{GlobalAlloc, Layout};

use spin::{Lazy, Mutex, RwLock};
use x86_64::instructions::interrupts;

use crate::proc::{Process, ProcessMode};

pub const HEAP_START_ADDR: usize = 0x400_0000;
pub const HEAP_SIZE: usize = 0x1000_0000;
pub const SAFETY_MARGIN: usize = 0x1_0000;

pub struct MemoryAllocator {
    arena: *mut u8,
    used: Lazy<Mutex<usize>>,
}

#[global_allocator]
static MEM_ALLOC: MemoryAllocator = MemoryAllocator {
    arena: HEAP_START_ADDR as *mut u8,
    used: Lazy::new(|| Mutex::new(0)),
};

static MEM_EXHAUSTED: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));

unsafe impl Sync for MemoryAllocator {}

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        interrupts::without_interrupts(|| {
            let mut used = self.used.lock();

            let size = layout.size();
            let align = layout.align();

            let start_offset = ((*used + align - 1) / align) * align;
            let end_offset = start_offset + size;

            if HEAP_SIZE - SAFETY_MARGIN <= end_offset {
                if !Self::exhausted() {
                    // Enter the safety mode.
                    let mut exhausted = MEM_EXHAUSTED.write();
                    *exhausted = true;

                    Process::switch_proc(ProcessMode::Safety);

                    // You may think that we should use `critical!` here.
                    // However, it can cause an unintentional deadlock because the memory allocation
                    // may be occurred during locking some resources which is required in safety mode.
                }
            }

            *used = end_offset;

            unsafe { self.arena.add(start_offset) }
        })
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing.
    }
}

impl MemoryAllocator {
    pub fn exhausted() -> bool {
        *MEM_EXHAUSTED.read()
    }
}
