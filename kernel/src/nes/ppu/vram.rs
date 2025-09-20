use heapless::Vec;
use serde::{Deserialize, Serialize};

const VRAM_OFFSET: usize = 0x2000;
const VRAM_SIZE: usize = 0x2000;

#[derive(Serialize, Deserialize)]
pub struct VRAM {
    mem: Vec<u8, VRAM_SIZE>,
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            mem: Vec::from_array([0; VRAM_SIZE]),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize - VRAM_OFFSET]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize - VRAM_OFFSET] = val;
    }
}
