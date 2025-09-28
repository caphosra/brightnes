use brightnes_common::serial::TriangleRequest;
use serde::{Deserialize, Serialize};

use crate::{
    error,
    nes::{apu::APU, cpu::CPU},
};

#[derive(Serialize, Deserialize)]
pub struct APUTriangle {
    pub active: bool,
    pub linear_counter: u8,
    pub linear_counter_control: bool,
    pub timer: u16,
    pub length_counter: u8,
}

impl APUTriangle {
    pub fn new() -> Self {
        Self {
            active: false,
            linear_counter: 0,
            linear_counter_control: false,
            timer: 0,
            length_counter: 0,
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) -> TriangleRequest {
        match addr {
            0 => {
                self.linear_counter = data & 0b0111_1111;
                self.linear_counter_control = ((data >> 7) & 1) != 0;
            }
            2 => {
                self.timer = (self.timer & 0xFF00) | (data as u16);
            }
            3 => {
                self.timer = (self.timer & 0x00FF) | (((data & 0b111) as u16) << 8);
                self.length_counter = (data >> 3) & 0b11111;
            }
            _ => {
                error!(
                    APU,
                    "Triangle does not support such operation: {:#06X}", addr
                );
            }
        }
        self.generate_request()
    }

    pub fn generate_request(&self) -> TriangleRequest {
        let frequency = CPU::CLOCK_FREQ as f64 / 32.0 / ((self.timer + 1) as f64);
        let length = if self.linear_counter_control {
            self.linear_counter as f64 * APU::QUARTER_FRAME_INTERVAL
        } else {
            APU::convert_length_counter(self.length_counter) as f64 * APU::HALF_FRAME_INTERVAL
        };
        TriangleRequest {
            active: self.active,
            frequency,
            length,
        }
    }
}
