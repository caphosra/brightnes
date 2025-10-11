use serde::{Deserialize, Serialize};
use spin::{Lazy, Once};

use crate::{
    critical,
    drivers::{SoundDeviceDriver, SAMPLING_RATE},
    log,
    mem::MemoryAllocator,
    nes::{
        apu::{dmc::DMC, noise::APUNoise, pulse::APUPulse, triangle::APUTriangle},
        cpu::{InterruptType, CPU},
    },
};

#[repr(u8)]
#[derive(Serialize, Deserialize)]
pub enum APUFrameCounterMode {
    FourStep = 0,
    FiveStep = 1,
}

#[derive(Serialize, Deserialize)]
pub struct APUFrameCounter {
    pub irq_disabled: bool,
    pub mode: APUFrameCounterMode,
    pub step: u32,
    pub frame: u8,
}

impl APUFrameCounter {
    pub fn new() -> Self {
        Self {
            irq_disabled: true,
            mode: APUFrameCounterMode::FourStep,
            step: 0,
            frame: 0,
        }
    }

    pub fn write_reg(&mut self, data: u8) {
        // SD-- ----

        self.mode = if ((data >> 7) & 1) != 0 {
            APUFrameCounterMode::FiveStep
        } else {
            APUFrameCounterMode::FourStep
        };

        let new_irq_disabled = ((data >> 6) & 1) == 0;
        if new_irq_disabled != self.irq_disabled {
            if new_irq_disabled {
                log!(APU, "APU Frame Counter IRQ disabled.");
            } else {
                log!(APU, "APU Frame Counter IRQ enabled.");
            }
            self.irq_disabled = new_irq_disabled;
        }

        self.step = 0;
        self.frame = 0;
    }
}

pub type SoundSampleType = i16;

trait APUComponent {
    fn write_reg(&mut self, addr: u16, data: u8);
    fn set_active(&mut self, active: bool);
    fn quarter_frame(&mut self);
    fn half_frame(&mut self);
    fn clock(&mut self, cycles: u32);
    fn get_output(&self) -> SoundSampleType;
}

#[derive(Serialize, Deserialize)]
pub struct APU {
    squares: [APUPulse; 2],
    triangle: APUTriangle,
    noise: APUNoise,
    dmc: DMC,
    frame_counter: APUFrameCounter,

    sampling_clocks_counter: u32,
}

static APU_PTR: Lazy<Once<usize>> = Lazy::new(|| Once::new());

impl APU {
    pub const QUARTER_FRAME_CLOCKS: u32 = CPU::CLOCK_FREQ / 240;

    pub const CPU_CLOCKS_PER_APU_CLOCK: u8 = 2;

    pub const SAMPLING_CLOCKS: u32 = CPU::CLOCK_FREQ / SAMPLING_RATE.to_hz();

    pub const VOLUME: SoundSampleType = 500;

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
            squares: [APUPulse::new(0), APUPulse::new(1)],
            triangle: APUTriangle::new(),
            noise: APUNoise::new(),
            dmc: DMC::new(),
            frame_counter: APUFrameCounter::new(),
            sampling_clocks_counter: 0,
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
                | (self.frame_counter.irq_disabled as u8) << 6
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

            self.squares[0].set_active((data & 1) != 0);
            self.squares[1].set_active(((data >> 1) & 1) != 0);
            self.triangle.set_active(((data >> 2) & 1) != 0);
            self.noise.active = ((data >> 3) & 1) != 0;
            self.dmc.active = ((data >> 4) & 1) != 0;
        } else if addr == 0x4017 {
            // Frame Counter
            self.frame_counter.write_reg(data);
        } else {
            critical!(APU, "Attempt to write unused register: {:#06X}", addr);
        }
    }

    pub fn clock(
        &mut self,
        cycles: u32,
        cpu: &mut CPU,
        sound: &mut SoundDeviceDriver<SoundSampleType>,
    ) {
        self.frame_counter.step += cycles;
        self.sampling_clocks_counter += cycles;

        self.squares[0].clock(cycles);
        self.squares[1].clock(cycles);
        self.triangle.clock(cycles);

        while self.frame_counter.step >= Self::QUARTER_FRAME_CLOCKS {
            // Quarter frame comes.

            self.frame_counter.step -= Self::QUARTER_FRAME_CLOCKS;
            self.frame_counter.frame += 1;
            match self.frame_counter.mode {
                APUFrameCounterMode::FourStep => {
                    if self.frame_counter.frame == 5 {
                        self.frame_counter.frame = 0;
                    }

                    if self.frame_counter.frame % 2 == 1 {
                        self.half_frame();
                    }
                    self.quarter_frame();

                    if self.frame_counter.frame == 0 && !self.frame_counter.irq_disabled {
                        // Trigger IRQ
                        cpu.interrupt(InterruptType::IRQ);
                    }
                }
                APUFrameCounterMode::FiveStep => {
                    if self.frame_counter.frame == 6 {
                        self.frame_counter.frame = 0;
                    }

                    if self.frame_counter.frame == 1 || self.frame_counter.frame == 4 {
                        self.half_frame();
                    }
                    if self.frame_counter.frame != 3 {
                        self.quarter_frame();
                    }
                }
            }
        }

        while self.sampling_clocks_counter >= Self::SAMPLING_CLOCKS {
            // Time to sample the sound data.

            self.sampling_clocks_counter -= Self::SAMPLING_CLOCKS;

            let output = (self.squares[0].get_output() as SoundSampleType) * Self::VOLUME
                + (self.squares[1].get_output() as SoundSampleType) * Self::VOLUME
                + (self.triangle.get_output() as SoundSampleType) * Self::VOLUME;
            sound.add_data(output, output);
        }
    }

    fn quarter_frame(&mut self) {
        self.squares[0].quarter_frame();
        self.squares[1].quarter_frame();
        self.triangle.quarter_frame();
    }

    fn half_frame(&mut self) {
        self.squares[0].half_frame();
        self.squares[1].half_frame();
        self.triangle.half_frame();
    }

    pub fn convert_length_counter(length: u8) -> u8 {
        match length {
            0x00 => 10,
            0x01 => 254,
            0x02 => 20,
            0x03 => 2,
            0x04 => 40,
            0x05 => 4,
            0x06 => 80,
            0x07 => 6,
            0x08 => 160,
            0x09 => 8,
            0x0A => 60,
            0x0B => 10,
            0x0C => 14,
            0x0D => 12,
            0x0E => 26,
            0x0F => 14,
            0x10 => 12,
            0x11 => 16,
            0x12 => 24,
            0x13 => 18,
            0x14 => 48,
            0x15 => 20,
            0x16 => 96,
            0x17 => 22,
            0x18 => 192,
            0x19 => 24,
            0x1A => 72,
            0x1B => 26,
            0x1C => 16,
            0x1D => 28,
            0x1E => 32,
            0x1F => 30,
            _ => {
                critical!(APU, "Invalid length counter value: {:#04X}", length);
            }
        }
    }
}

pub mod dmc;
pub mod noise;
pub mod pulse;
pub mod triangle;
