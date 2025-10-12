use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::{
        apu::{SoundSampleType, APU},
        cartridge::Cartridge,
        cpu::{bus::CPUBus, InterruptType, CPU},
        ppu::PPU,
    },
};

#[derive(Serialize, Deserialize)]
pub struct DMC {
    pub irq: bool,
    loop_enabled: bool,
    sample_address: u16,
    sample_length: u16,
    timer: u16,

    current_address: u16,
    length_counter: u16,

    sample_buffer: u8,
    sample_buffer_index: u8,

    timer_counter: u16,
    output: u8,
}

impl DMC {
    const RATE_TABLE: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];

    pub fn new() -> Self {
        Self {
            irq: false,
            loop_enabled: false,
            timer: 0,
            sample_address: 0,
            sample_length: 0,

            current_address: 0,
            length_counter: 0,

            sample_buffer: 0,
            sample_buffer_index: 0,

            timer_counter: 0,
            output: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.length_counter > 0
    }

    pub fn set_active(&mut self, active: bool) {
        if !active {
            self.length_counter = 0;
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.irq = ((data >> 7) & 1) != 0;
                self.loop_enabled = ((data >> 6) & 1) != 0;
                self.timer = Self::RATE_TABLE[(data & 0b1111) as usize];
            }
            1 => {
                self.output = data & 0x7F;
            }
            2 => {
                self.sample_address = 0xC000 + ((data as u16) << 6);
            }
            3 => {
                self.sample_length = ((data as u16) << 4) + 1;
                self.length_counter = self.sample_length;
            }
            _ => {
                critical!(APU, "DMC does not support such operation: {:#06X}", addr);
            }
        }
    }

    pub fn clock(&mut self, cycles: u32, cpu: &mut CPU, ppu: &mut PPU, cartridge: &mut Cartridge) {
        if self.timer == 0 {
            return;
        }

        let apu = APU::get();

        self.timer_counter += cycles as u16;
        while self.timer_counter >= self.timer {
            self.timer_counter -= self.timer;

            if self.sample_buffer_index == 0 {
                // Sample buffer is empty.
                if self.length_counter > 0 {
                    self.length_counter -= 1;

                    // Read sample data.
                    self.sample_buffer =
                        CPUBus::read(self.current_address, cpu, ppu, apu, cartridge);
                    self.sample_buffer_index = 8;

                    if self.current_address == 0xFFFF {
                        self.current_address = 0x8000;
                    } else {
                        self.current_address += 1;
                    }
                } else if self.loop_enabled {
                    // Restart sample.
                    self.current_address = self.sample_address;
                    self.length_counter = self.sample_length;
                } else if self.irq {
                    // Trigger DMC IRQ.
                    cpu.interrupt(InterruptType::IRQ);
                }
            } else {
                // Output sample bit.
                if (self.sample_buffer & 1) != 0 {
                    if self.output < 126 {
                        self.output += 2;
                    }
                } else {
                    if self.output > 1 {
                        self.output -= 2;
                    }
                }

                self.sample_buffer >>= 1;
                self.sample_buffer_index -= 1;
            }
        }
    }

    pub fn get_output(&self) -> SoundSampleType {
        self.output as SoundSampleType
    }
}
