use brightnes_common::serial::{PulseRequest, Volume};
use serde::{Deserialize, Serialize};

use crate::{critical, nes::apu::APU};

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

    pub fn write_reg(&mut self, addr: u16, data: u8) -> PulseRequest {
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
        self.generate_request()
    }

    pub fn generate_request(&self) -> PulseRequest {
        let frequency = APU::CPU_CLOCK_FREQUENCY as f64 / (16.0 * (self.timer as f64 + 1.0));
        let volume = if self.constant_volume {
            // Constant volume
            Volume::Constant(self.volume as f64 / 15.0)
        } else {
            // Decreasing volume over time
            if self.volume == 0 {
                Volume::Decreasing(f64::INFINITY)
            } else {
                Volume::Decreasing(self.volume as f64 * APU::QUARTER_FRAME_INTERVAL)
            }
        };
        let length = if self.loop_enabled {
            f64::INFINITY
        } else {
            APU::convert_length_counter(self.length_counter) as f64 * APU::QUARTER_FRAME_INTERVAL
        };
        let duty_rate = match self.duty_cycle {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => {
                critical!(APU, "Invalid duty cycle: {}", self.duty_cycle);
            }
        };
        let sweep_interval = if self.sweep_enabled {
            self.sweep_period as f64 * APU::HALF_FRAME_INTERVAL
        } else {
            f64::INFINITY
        };
        PulseRequest {
            active: self.active,
            frequency,
            volume,
            length,
            loop_enabled: self.loop_enabled,
            duty_rate,
            sweep_enabled: self.sweep_enabled,
            sweep_interval,
            sweep_negate: self.sweep_negate,
            sweep_shift: self.sweep_shift,
        }
    }
}
