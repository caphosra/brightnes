use heapless::Vec;
use serde::{Deserialize, Serialize};

const RAM_SIZE: usize = 0x800;

#[derive(Serialize, Deserialize)]
pub struct RAM {
    mem: Vec<u8, RAM_SIZE>,
}

impl RAM {
    pub fn new() -> Self {
        RAM {
            mem: Vec::from_array([0; RAM_SIZE]),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
    }
}
