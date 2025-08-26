use spin::{Lazy, RwLock};

const RAM_SIZE: usize = 0x800;

pub struct NESRAM {
    pub ram: [u8; RAM_SIZE],
}

pub static NES_RAM: Lazy<RwLock<NESRAM>> = Lazy::new(|| RwLock::new(NESRAM { ram: [0; RAM_SIZE] }));
