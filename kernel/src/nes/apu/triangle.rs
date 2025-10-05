use brightnes_common::serial::{APURequest, TriangleRequest};
use serde::{Deserialize, Serialize};

use crate::{
    error,
    nes::{
        apu::{APUComponent, APU},
        cpu::CPU,
    },
    serial::Serial,
};

#[derive(Serialize, Deserialize)]
pub struct APUTriangle {
    pub active: bool,
    pub linear_counter: u8,
    pub linear_counter_control: bool,
    pub timer: u16,
    pub length_counter: u8,

    pub last_active: bool,
    pub last_timer: u16,
}

impl APUComponent for APUTriangle {
    fn write_reg(&mut self, addr: u16, data: u8) {
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
                self.length_counter = APU::convert_length_counter((data >> 3) & 0b11111);
            }
            _ => {
                error!(
                    APU,
                    "Triangle does not support such operation: {:#06X}", addr
                );
            }
        }
    }

    fn set_active(&mut self, active: bool) {
        if self.active != active {
            if !active {
                self.length_counter = 0;
            }
            self.active = active;
            self.send_request();
        }
    }

    fn quarter_frame(&mut self) {
        if self.linear_counter_control && self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        self.send_request();
    }

    fn half_frame(&mut self) {
        if !self.linear_counter_control && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn clock(&mut self, _cycles: u32) {}

    fn get_output(&self) -> i8 {
        0
    }
}

impl APUTriangle {
    pub fn new() -> Self {
        Self {
            active: false,
            linear_counter: 0,
            linear_counter_control: false,
            timer: 0,
            length_counter: 0,

            last_active: false,
            last_timer: 0,
        }
    }

    fn send_request(&mut self) {
        let counter_ok = (self.linear_counter_control && self.linear_counter > 0)
            || (!self.linear_counter_control && self.length_counter > 0);
        let active = self.active && counter_ok && self.length_counter > 0 && self.timer >= 8;

        if self.last_active != active || (active && self.last_timer != self.timer) {
            // There are some changes, send a request.

            let frequency = if self.timer == 0 {
                0.0
            } else {
                CPU::CLOCK_FREQ as f64 / (32 * (self.timer + 1)) as f64
            };
            let request = TriangleRequest { active, frequency };

            Serial::communicate(|handler| handler.request_sound(APURequest::Triangle(request)));

            self.last_active = active;
            self.last_timer = self.timer;
        }
    }
}
