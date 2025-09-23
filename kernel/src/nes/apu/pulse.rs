use serde::{Deserialize, Serialize};

use crate::critical;

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
