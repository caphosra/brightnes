use serde::{Deserialize, Serialize};
use spin::{Lazy, Once};

use crate::{
    critical, log,
    mem::MemoryAllocator,
    nes::cpu::{InterruptType, CPU},
};

#[derive(Serialize, Deserialize)]
pub struct APUPulse {
    pub active: bool,
    pub volume: u8,
    pub constant_volume: bool,
    pub loop_enabled: bool,
    pub duty_cycle: u8,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub timer: u16,
    pub length_counter: u8,
}

impl APUPulse {
    pub fn new() -> Self {
        Self {
            active: false,
            volume: 0,
            constant_volume: false,
            loop_enabled: false,
            duty_cycle: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            timer: 0,
            length_counter: 0,
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.duty_cycle = (data >> 6) & 0b11;
                self.loop_enabled = ((data >> 5) & 1) != 0;
                self.constant_volume = ((data >> 4) & 1) != 0;
                self.volume = data & 0b1111;
            }
            1 => {
                self.sweep_enabled = ((data >> 7) & 1) != 0;
                self.sweep_period = (data >> 4) & 0b111;
                self.sweep_negate = ((data >> 3) & 1) != 0;
                self.sweep_shift = data & 0b111;
            }
            2 => {
                self.timer = (self.timer & 0xFF00) | (data as u16);
            }
            3 => {
                self.timer = (self.timer & 0x00FF) | (((data & 0b111) as u16) << 8);
                self.length_counter = (data >> 3) & 0b11111;
            }
            _ => {
                critical!(APU, "Pulse does not support such operation: {:#06X}", addr);
            }
        }
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

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
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

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
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

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
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
    step: usize,
    frame: u8,
}

impl APUFrameCounter {
    const CLOCK_PER_FRAME: usize = 7457;

    pub fn new() -> Self {
        Self {
            irq: false,
            mode: APUFrameCounterMode::FourStep,
            step: 0,
            frame: 0,
        }
    }

    pub fn clock(&mut self, cycles: usize, cpu: &mut CPU) {
        self.step += cycles;
        if self.step >= Self::CLOCK_PER_FRAME {
            self.step -= Self::CLOCK_PER_FRAME;
            self.frame += 1;
            match self.mode {
                APUFrameCounterMode::FourStep => {
                    if self.frame == 5 {
                        self.frame = 0;
                    }

                    if self.frame == 0 && self.irq {
                        // Trigger IRQ
                        cpu.interrupt(InterruptType::IRQ);
                    }
                }
                APUFrameCounterMode::FiveStep => {
                    if self.frame == 6 {
                        self.frame = 0;
                    }
                }
            }
        }
    }

    pub fn write_reg(&mut self, data: u8) {
        // SD-- ----

        self.mode = if ((data >> 7) & 1) != 0 {
            APUFrameCounterMode::FiveStep
        } else {
            APUFrameCounterMode::FourStep
        };

        let frame_counter_irq = ((data >> 6) & 1) == 0;
        if frame_counter_irq != self.irq {
            if frame_counter_irq {
                log!(APU, "APU Frame Counter IRQ enabled.");
            } else {
                log!(APU, "APU Frame Counter IRQ disabled.");
            }
            self.irq = frame_counter_irq;
        }

        self.step = 0;
        self.frame = 0;
    }
}

#[derive(Serialize, Deserialize)]
pub struct APU {
    squares: [APUPulse; 2],
    triangle: APUTriangle,
    noise: APUNoise,
    dmc: DMC,
    frame_counter: APUFrameCounter,
}

static APU_PTR: Lazy<Once<usize>> = Lazy::new(|| Once::new());

impl APU {
    pub fn get() -> &'static mut Self {
        let ptr = *APU_PTR.call_once(|| {
            // Allocate memory for the APU.
            let apu_raw_ptr = MemoryAllocator::alloc_zeroed::<APU>();
            apu_raw_ptr as usize
        }) as *mut APU;
        unsafe { ptr.as_mut() }.unwrap()
    }

    pub fn init(&mut self) {
        *self = Self {
            squares: [APUPulse::new(), APUPulse::new()],
            triangle: APUTriangle::new(),
            noise: APUNoise::new(),
            dmc: DMC::new(),
            frame_counter: APUFrameCounter::new(),
        };
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
            critical!(APU, "Attempt to read unused register: {:#06X}", addr);
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
            // Frame Counter
            self.frame_counter.write_reg(data);
        } else {
            critical!(APU, "Attempt to write unused register: {:#06X}", addr);
        }
    }

    pub fn clock(&mut self, cycles: usize, cpu: &mut CPU) {
        self.frame_counter.clock(cycles, cpu);
    }
}

pub mod bus;
