const VRAM_OFFSET: usize = 0x2000;
const VRAM_SIZE: usize = 0x2000;

pub struct VRAM {
    mem: [u8; VRAM_SIZE],
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            mem: [0; VRAM_SIZE],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize - VRAM_OFFSET]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize - VRAM_OFFSET] = val;
    }
}
