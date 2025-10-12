use serde::{Deserialize, Serialize};

use crate::{
    error,
    nes::apu::{APUChannel, SoundSampleType, APU},
};

#[derive(Serialize, Deserialize)]
pub struct APUTriangle {
    pub linear_counter: u8,
    pub linear_counter_control: bool,
    pub timer: u16,
    pub length_counter: u8,

    linear_counter_reload_value: u8,
    linear_counter_reload: bool,

    timer_counter: u16,
    duty_step: u8,
}

impl APUChannel for APUTriangle {
    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.linear_counter_reload_value = data & 0b0111_1111;
                self.linear_counter_control = ((data >> 7) & 1) != 0;
            }
            2 => {
                self.timer = (self.timer & 0xFF00) | (data as u16);
            }
            3 => {
                self.timer = (self.timer & 0x00FF) | (((data & 0b111) as u16) << 8);
                self.length_counter = APU::convert_length_counter((data >> 3) & 0b11111);

                self.linear_counter_reload = true;
            }
            _ => {
                error!(
                    APU,
                    "Triangle does not support such operation: {:#06X}", addr
                );
            }
        }
    }

    fn active(&self) -> bool {
        self.length_counter > 0 && self.linear_counter > 0
    }

    fn set_active(&mut self, active: bool) {
        if !active {
            self.length_counter = 0;
            self.linear_counter = 0;
        }
    }

    fn quarter_frame(&mut self) {
        if self.linear_counter_reload {
            self.linear_counter = self.linear_counter_reload_value;
        } else if self.linear_counter_control && self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        if !self.linear_counter_control {
            self.linear_counter_reload = false;
        }
    }

    fn half_frame(&mut self) {
        if !self.linear_counter_control && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn clock(&mut self, cycles: u32) {
        self.timer_counter += cycles as u16;
        self.duty_step =
            (self.duty_step + (self.timer_counter / (self.timer + 1)) as u8) % Self::MAX_DUTY_STEPS;
        self.timer_counter %= self.timer + 1;
    }

    fn get_output(&self) -> SoundSampleType {
        if self.linear_counter > 0 && self.length_counter > 0 && self.timer >= 2 {
            let output = if self.duty_step & 0x10 != 0 {
                self.duty_step ^ 0x1F
            } else {
                self.duty_step
            };
            output as SoundSampleType - 7
        } else {
            0
        }
    }
}

impl APUTriangle {
    const MAX_DUTY_STEPS: u8 = 32;

    pub fn new() -> Self {
        Self {
            linear_counter: 0,
            linear_counter_control: false,
            timer: 0,
            length_counter: 0,

            linear_counter_reload_value: 0,
            linear_counter_reload: false,

            timer_counter: 0,
            duty_step: 0,
        }
    }
}
