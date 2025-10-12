use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::apu::{APUChannel, SoundSampleType, APU},
};

#[derive(Serialize, Deserialize)]
pub struct APUPulse {
    id: usize,

    pub const_volume: u8,
    pub envelope_volume: u8,
    pub const_volume_enabled: bool,
    pub loop_enabled: bool,
    pub duty_cycle: u8,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub timer: u16,
    pub length_counter: u8,

    envelope_period: u8,
    envelope_counter: u8,
    envelope_reload: bool,

    sweep_counter: u8,

    timer_counter: u16,
    duty_step: u8,
}

impl APUChannel for APUPulse {
    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.duty_cycle = (data >> 6) & 0b11;
                self.loop_enabled = ((data >> 5) & 1) != 0;
                self.const_volume_enabled = ((data >> 4) & 1) != 0;

                if self.const_volume_enabled {
                    self.const_volume = data & 0b1111;
                } else {
                    self.envelope_period = data & 0b1111;
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
                self.envelope_reload = true;
            }
            _ => {
                critical!(APU, "Pulse does not support such operation: {:#06X}", addr);
            }
        };
    }

    fn active(&self) -> bool {
        self.length_counter > 0
    }

    fn set_active(&mut self, active: bool) {
        if !active {
            // Reset length counter to stop playing the sound.
            self.length_counter = 0;
        }
    }

    fn quarter_frame(&mut self) {
        // Envelope
        if self.envelope_reload {
            // Reset the envelope.
            self.envelope_reload = false;
            self.envelope_volume = Self::MAX_VOLUME;
            self.envelope_counter = self.envelope_period;
        } else {
            if self.envelope_counter > 0 {
                // Decrement the envelope counter.
                self.envelope_counter -= 1;
            } else {
                // Update the volume.
                self.envelope_counter = self.envelope_period;
                if self.envelope_volume > 0 {
                    self.envelope_volume -= 1;
                } else {
                    if self.loop_enabled {
                        // Reset the volume if looping is enabled.
                        self.envelope_volume = Self::MAX_VOLUME;
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
                if self.timer >= 8 && self.timer <= 0x7FF {
                    // Perform the sweep.
                    let change = self.timer >> self.sweep_shift;
                    if self.sweep_negate {
                        self.timer = self.timer.checked_sub(change).unwrap_or(0);
                    } else {
                        self.timer = self.timer.wrapping_add(change);
                    }
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
        if self.length_counter > 0 && self.timer >= 8 && self.timer <= 0x7FF {
            let output = if self.output_duty() {
                if self.const_volume_enabled {
                    self.const_volume
                } else {
                    self.envelope_volume
                }
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

            envelope_volume: 0,
            const_volume: 0,
            const_volume_enabled: false,
            loop_enabled: false,
            duty_cycle: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            timer: 0,
            length_counter: 0,

            sweep_counter: 0,
            envelope_period: 0,
            envelope_counter: 0,
            envelope_reload: false,

            timer_counter: 0,
            duty_step: 0,
        }
    }

    pub fn output_duty(&self) -> bool {
        match self.duty_cycle {
            0 => self.duty_step == 7,
            1 => self.duty_step >= 6,
            2 => self.duty_step >= 4,
            3 => self.duty_step < 6,
            _ => {
                critical!(APU, "Invalid duty cycle: {}", self.duty_cycle);
            }
        }
    }
}
