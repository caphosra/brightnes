use core::alloc::{GlobalAlloc, Layout};

use spin::{Lazy, Mutex, RwLock};
use x86_64::instructions::interrupts;

use crate::proc::{Process, ProcessMode};

pub const HEAP_START_ADDR: usize = 0x4_000_000;
#[allow(dead_code)]
pub const HEAP_SIZE: usize = 0x200_0000;
pub const SAFETY_MARGIN: usize = 0x1FF_0000;

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
            if end_offset >= SAFETY_MARGIN {
                if !Self::exhausted() {
                    // Enter the safety mode.
                    let mut exhausted = MEM_EXHAUSTED.write();
                    *exhausted = true;

                    Process::switch_proc(ProcessMode::Recovery);

                    // You may think that we should use `critical!` here.
                    // However, it can cause an unintentional deadlock because the memory allocation
                    // may be occurred during locking some resources which is required in safety mode.
                }
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
    pub fn exhausted() -> bool {
        *MEM_EXHAUSTED.read()
    }
}
