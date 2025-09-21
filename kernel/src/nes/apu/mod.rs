use serde::{Deserialize, Serialize};
use spin::{Lazy, RwLock};

use crate::{log, warn};

#[derive(Serialize, Deserialize)]
pub struct APUSquare {
    pub active: bool,
}

impl APUSquare {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        // TODO
    }
}

#[derive(Serialize, Deserialize)]
pub struct APUTriangle {
    pub active: bool,
}

impl APUTriangle {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        // TODO
    }
}

#[derive(Serialize, Deserialize)]
pub struct APUNoise {
    pub active: bool,
}

impl APUNoise {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        // TODO
    }
}

#[derive(Serialize, Deserialize)]
pub struct DMC {
    pub active: bool,
    pub irq: bool,
}

impl DMC {
    pub fn new() -> Self {
        Self {
            active: false,
            irq: false,
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        // TODO
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize)]
pub enum APUFrameCounterMode {
    FourStep = 0,
    FiveStep = 1,
}

#[derive(Serialize, Deserialize)]
pub struct APUFrameCounter {
    pub irq: bool,
    pub mode: APUFrameCounterMode,
}

impl APUFrameCounter {
    pub fn new() -> Self {
        Self {
            irq: false,
            mode: APUFrameCounterMode::FourStep,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct APU {
    squares: [APUSquare; 2],
    triangle: APUTriangle,
    noise: APUNoise,
    dmc: DMC,
    frame_counter: APUFrameCounter,
}

pub static NES_APU: Lazy<RwLock<APU>> = Lazy::new(|| RwLock::new(APU::new()));

impl APU {
    pub fn new() -> Self {
        Self {
            squares: [APUSquare::new(), APUSquare::new()],
            triangle: APUTriangle::new(),
            noise: APUNoise::new(),
            dmc: DMC::new(),
            frame_counter: APUFrameCounter::new(),
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        if addr == 0x4015 {
            // IF-D NT21

            (self.squares[0].active) as u8
                | (self.squares[1].active as u8) << 1
                | (self.triangle.active as u8) << 2
                | (self.noise.active as u8) << 3
                | (self.dmc.active as u8) << 4
                | (self.frame_counter.irq as u8) << 6
                | (self.dmc.irq as u8) << 7
        } else {
            warn!(
                APU,
                "Attempt to read unimplemented APU register: {:#06X}", addr
            );
            0
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        if addr < 0x4004 {
            // Square 1
            self.squares[0].write_reg(addr - 0x4000, data);
        } else if addr < 0x4008 {
            // Square 2
            self.squares[1].write_reg(addr - 0x4004, data);
        } else if addr < 0x400C {
            // Triangle
            self.triangle.write_reg(addr - 0x4008, data);
        } else if addr < 0x4010 {
            // Noise
            self.noise.write_reg(addr - 0x400C, data);
        } else if addr < 0x4014 {
            // DMC
            self.dmc.write_reg(addr - 0x4010, data);
        } else if addr == 0x4015 {
            // ---D NT21

            self.squares[0].active = (data & 1) != 0;
            self.squares[1].active = ((data >> 1) & 1) != 0;
            self.triangle.active = ((data >> 2) & 1) != 0;
            self.noise.active = ((data >> 3) & 1) != 0;
            self.dmc.active = ((data >> 4) & 1) != 0;
        } else if addr == 0x4017 {
            // SD-- ----

            self.frame_counter.mode = if ((data >> 7) & 1) != 0 {
                APUFrameCounterMode::FiveStep
            } else {
                APUFrameCounterMode::FourStep
            };

            let frame_counter_irq = ((data >> 6) & 1) == 0;
            if frame_counter_irq != self.frame_counter.irq {
                if frame_counter_irq {
                    log!(APU, "APU Frame Counter IRQ enabled.");
                } else {
                    log!(APU, "APU Frame Counter IRQ disabled.");
                }
                self.frame_counter.irq = frame_counter_irq;
            }
        } else {
            warn!(
                APU,
                "Attempt to write unimplemented APU register: {:#06X}", addr
            );
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        for _ in 0..cycles {
            // TODO
        }
    }
}

pub mod bus;
