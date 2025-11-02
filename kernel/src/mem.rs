use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    cmp::max,
    ptr::{null_mut, write_bytes},
    sync::atomic::{AtomicBool, Ordering},
};

use spin::{Lazy, Mutex};
use x86_64::instructions::interrupts;

pub const HEAP_START_ADDR: usize = 0x1000_0000;
pub const HEAP_SAFE_MARGIN: usize = 0x1000;
pub const HEAP_SIZE: usize = 0x2000_0000;

#[repr(C)]
struct Chunk {
    next: *mut Chunk,
}

#[repr(C)]
struct ChunkList {
    first_chunk: UnsafeCell<*mut Chunk>,
}

impl ChunkList {
    pub const fn new() -> Self {
        Self {
            first_chunk: UnsafeCell::new(null_mut()),
        }
    }

    pub fn init(&self) {
        let first_chunk = unsafe { self.first_chunk.get().as_mut() }.unwrap();
        *first_chunk = null_mut();
    }

    pub fn get_mem(&self) -> Option<*mut u8> {
        let chunk_ptr = unsafe { self.first_chunk.get().as_mut() }.unwrap();
        let chunk = *chunk_ptr;

        if chunk.is_null() {
            None
        } else {
            let next_chunk = unsafe { (*chunk).next };
            *chunk_ptr = next_chunk;
            Some(chunk as *mut u8)
        }
    }

    pub fn insert_mem(&self, ptr: *mut u8) {
        let first_chunk = unsafe { self.first_chunk.get().as_mut() }.unwrap();

        let new_chunk = ptr as *mut Chunk;
        unsafe {
            (*new_chunk).next = *first_chunk;
        }
        *first_chunk = new_chunk;
    }
}

unsafe impl Send for ChunkList {}
unsafe impl Sync for ChunkList {}

#[repr(C)]
pub struct ReleasedMem {
    chunk_lists: [ChunkList; Self::NUM_CLASSES],
}

unsafe impl Send for ReleasedMem {}
unsafe impl Sync for ReleasedMem {}

static RELEASED_MEM: ReleasedMem = ReleasedMem::new();

impl ReleasedMem {
    pub const MAX_BITS: usize = 16;
    pub const NUM_CLASSES: usize = Self::MAX_BITS - MemoryAllocator::MIN_BITS + 1;

    pub const MAX_SIZE: usize = 1 << Self::MAX_BITS;

    pub const fn new() -> Self {
        Self {
            chunk_lists: [const { ChunkList::new() }; Self::NUM_CLASSES],
        }
    }

    pub fn init(&self) {
        for chunk_list in self.chunk_lists.iter() {
            chunk_list.init();
        }
    }

    fn index_by_size(&self, size: usize) -> Option<usize> {
        // Ignore larger sizes.
        if size > Self::MAX_SIZE {
            None
        } else {
            let bits = size.trailing_zeros() as usize;
            Some(bits - MemoryAllocator::MIN_BITS)
        }
    }

    pub fn get_mem(&self, size: usize) -> Option<*mut u8> {
        let index = self.index_by_size(size)?;
        self.chunk_lists[index].get_mem()
    }

    pub fn insert_mem(&self, ptr: *mut u8, size: usize) {
        if let Some(index) = self.index_by_size(size) {
            self.chunk_lists[index].insert_mem(ptr);
        }
    }
}

pub struct MemoryAllocator {
    arena: *mut u8,
    used: Lazy<Mutex<usize>>,
    mem_error_notified: AtomicBool,
}

#[global_allocator]
static MEM_ALLOC: MemoryAllocator = MemoryAllocator {
    arena: HEAP_START_ADDR as *mut u8,
    used: Lazy::new(|| Mutex::new(0)),
    mem_error_notified: AtomicBool::new(false),
};

unsafe impl Sync for MemoryAllocator {}

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Interrupt during memory allocation can corrupt the memory.
        // Also, it can cause a deadlock due to the mutexes.
        let int_enabled = interrupts::are_enabled();
        interrupts::disable();

        let size = max(layout.size().next_power_of_two(), MemoryAllocator::MIN_SIZE);

        // Try to get memory from chunk lists.
        if let Some(mem) = RELEASED_MEM.get_mem(size) {
            if mem as usize % layout.align() == 0 {
                // The memory is following the alignment constraint.
                if int_enabled {
                    interrupts::enable();
                }
                return mem;
            } else {
                // Re-insert the memory to chunk lists.
                RELEASED_MEM.insert_mem(mem, size);
            }
        }

        // Allocate memory from the arena.
        let allocated = {
            let mut used = self.used.lock();

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

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let int_enabled = interrupts::are_enabled();
        interrupts::disable();

        // Assume that the size is a power of two.
        let size = max(layout.size().next_power_of_two(), MemoryAllocator::MIN_SIZE);
        RELEASED_MEM.insert_mem(ptr, size);

        if int_enabled {
            interrupts::enable();
        }
    }
}

impl MemoryAllocator {
    pub const MIN_BITS: usize = 6;

    pub const MIN_SIZE: usize = 1 << Self::MIN_BITS;

    pub fn init() {
        interrupts::without_interrupts(|| {
            RELEASED_MEM.init();
        });
    }

    pub fn check_mem_error() -> bool {
        MEM_ALLOC
            .mem_error_notified
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |notified| {
                if !notified {
                    let used = MEM_ALLOC.used.lock();
                    if *used >= HEAP_SIZE - HEAP_SAFE_MARGIN {
                        return Some(true);
                    }
                }
                None
            })
            .is_ok()
    }

    pub fn alloc<T>() -> *mut T {
        let allocated = unsafe { MEM_ALLOC.alloc(Layout::new::<T>()) };
        allocated as *mut T
    }

    pub fn alloc_zeroed<T>() -> *mut T {
        let ptr = Self::alloc::<T>();
        unsafe { write_bytes(ptr, 0, 1) };
        ptr
    }
}
