use brightnes_common::serial::APURequest;
use serde::{Deserialize, Serialize};
use spin::{Lazy, Once};

use crate::{
    critical, log,
    mem::MemoryAllocator,
    nes::{
        apu::{dmc::DMC, noise::APUNoise, pulse::APUPulse, triangle::APUTriangle},
        cpu::{InterruptType, CPU},
    },
    serial::Serial,
};

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
    pub const CPU_CLOCK_FREQUENCY: usize = 1789773;
    pub const QUARTER_FRAME_INTERVAL: f64 = 1.0 / 60.0 / 4.0;

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
            let req = self.squares[0].write_reg(addr - 0x4000, data);
            Serial::communicate(|handler| handler.request_sound(APURequest::Pulse(0, req)));
        } else if addr < 0x4008 {
            // Square 2
            let req = self.squares[1].write_reg(addr - 0x4004, data);
            Serial::communicate(|handler| handler.request_sound(APURequest::Pulse(1, req)));
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
pub mod dmc;
pub mod noise;
pub mod pulse;
pub mod triangle;
