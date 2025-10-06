use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::apu::{APUComponent, SoundSampleType, APU},
};

#[derive(Serialize, Deserialize)]
pub struct APUPulse {
    id: usize,

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

    volume_period: u8,
    volume_counter: u8,

    sweep_counter: u8,

    last_active: bool,
    last_volume: u8,
    last_time: u16,
    last_duty_cycle: u8,

    timer_counter: u16,
    duty_step: u8,
}

impl APUComponent for APUPulse {
    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.duty_cycle = (data >> 6) & 0b11;
                self.loop_enabled = ((data >> 5) & 1) != 0;
                self.constant_volume = ((data >> 4) & 1) != 0;

                if self.constant_volume {
                    self.volume = data & 0b1111;
                } else {
                    self.volume = Self::MAX_VOLUME;
                    self.volume_period = data & 0b1111;
                    self.volume_counter = self.volume_period;
                }
            }
            1 => {
                self.sweep_enabled = ((data >> 7) & 1) != 0;
                self.sweep_period = (data >> 4) & 0b111;
                self.sweep_negate = ((data >> 3) & 1) != 0;
                self.sweep_shift = data & 0b111;

                self.sweep_counter = self.sweep_period;
            }
            2 => {
                self.timer = (self.timer & 0xFF00) | (data as u16);
            }
            3 => {
                self.timer = (self.timer & 0x00FF) | (((data & 0b111) as u16) << 8);
                self.length_counter = APU::convert_length_counter((data >> 3) & 0b11111);

                // Reset the volume envelope
                if !self.constant_volume {
                    self.volume = Self::MAX_VOLUME;
                    self.volume_counter = self.volume_period;
                }
            }
            _ => {
                critical!(APU, "Pulse does not support such operation: {:#06X}", addr);
            }
        };
    }

    fn set_active(&mut self, active: bool) {
        if self.active != active {
            if !active {
                // Reset length counter to stop playing the sound.
                self.length_counter = 0;
            }
            self.active = active;
        }
    }

    fn quarter_frame(&mut self) {
        // Envelope
        if !self.constant_volume {
            if self.volume_counter > 0 {
                self.volume_counter -= 1;
            } else {
                self.volume_counter = self.volume_period;
                if self.volume > 0 {
                    self.volume -= 1;
                } else {
                    if self.loop_enabled {
                        // Reset the volume if looping is enabled.
                        self.volume = Self::MAX_VOLUME;
                    }
                }
            }
        }
    }

    fn half_frame(&mut self) {
        // Sweep
        if self.sweep_counter > 0 {
            self.sweep_counter -= 1;
        } else {
            self.sweep_counter = self.sweep_period;
            if self.sweep_enabled && self.sweep_shift > 0 {
                let change = self.timer >> self.sweep_shift;
                if self.sweep_negate {
                    self.timer = self.timer.checked_sub(change).unwrap_or(0);
                } else {
                    self.timer = self.timer.wrapping_add(change);
                }
                if self.timer > 0x7FF {
                    // Mute the channel
                    self.timer = 0;
                }
            }
        }

        // Length counter
        if !self.loop_enabled {
            if self.length_counter > 0 {
                self.length_counter -= 1;
            }
        }
    }

    fn clock(&mut self, cycles: u32) {
        self.timer_counter += cycles as u16 / APU::CPU_CLOCKS_PER_APU_CLOCK as u16;
        self.duty_step =
            (self.duty_step + (self.timer_counter / (self.timer + 1)) as u8) % Self::MAX_DUTY_STEPS;
        self.timer_counter %= self.timer + 1;
    }

    fn get_output(&self) -> SoundSampleType {
        if self.active && self.length_counter > 0 && self.timer >= 8 && self.volume > 0 {
            let duty_rate = self.duty_rate();
            let output = if (self.duty_step as f64) < (Self::MAX_DUTY_STEPS as f64 * duty_rate) {
                self.volume
            } else {
                0
            };
            output as SoundSampleType - 7
        } else {
            0
        }
    }
}

impl APUPulse {
    const MAX_VOLUME: u8 = 0x0F;
    const MAX_DUTY_STEPS: u8 = 8;

    pub fn new(id: usize) -> Self {
        Self {
            id,

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

            sweep_counter: 0,
            volume_period: 0,
            volume_counter: 0,

            last_active: false,
            last_volume: 0,
            last_time: 0,
            last_duty_cycle: 0,

            timer_counter: 0,
            duty_step: 0,
        }
    }

    pub fn duty_rate(&self) -> f64 {
        match self.duty_cycle {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => {
                critical!(APU, "Invalid duty cycle: {}", self.duty_cycle);
            }
        }
    }
}
