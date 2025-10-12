use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::apu::{APUChannel, SoundSampleType, APU},
};

#[derive(Serialize, Deserialize)]
pub struct APUNoise {
    const_volume: u8,
    envelope_volume: u8,
    const_volume_enabled: bool,
    loop_enabled: bool,
    length_counter: u8,
    short_mode: bool,
    timer: u16,

    shift_register: u16,
    timer_counter: u16,

    envelope_period: u8,
    envelope_counter: u8,
    envelope_reload: bool,
}

impl APUChannel for APUNoise {
    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.loop_enabled = ((data >> 5) & 1) != 0;
                self.const_volume_enabled = ((data >> 4) & 1) != 0;

                if self.const_volume_enabled {
                    self.const_volume = data & 0b1111;
                } else {
                    self.envelope_period = data & 0b1111;
                }
            }
            2 => {
                self.short_mode = ((data >> 7) & 1) != 0;
                self.timer = Self::NOISE_PERIODS[(data & 0b1111) as usize];
            }
            3 => {
                self.length_counter = APU::convert_length_counter((data >> 3) & 0b11111);

                self.envelope_reload = true;
            }
            _ => {
                critical!(APU, "Noise does not support such operation: {:#06X}", addr);
            }
        }
    }

    fn active(&self) -> bool {
        self.length_counter > 0
    }

    fn set_active(&mut self, active: bool) {
        if !active {
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
        if self.length_counter > 0 && !self.loop_enabled {
            self.length_counter -= 1;
        }
    }

    fn clock(&mut self, cycles: u32) {
        self.timer_counter += cycles as u16;
        if self.timer_counter >= self.timer {
            self.timer_counter -= self.timer;

            let feedback = if self.short_mode {
                // Short mode
                (self.shift_register & 1) ^ ((self.shift_register >> 6) & 1)
            } else {
                // Long mode
                (self.shift_register & 1) ^ ((self.shift_register >> 1) & 1)
            };

            self.shift_register = (self.shift_register >> 1) | (feedback << 14);
        }
    }

    fn get_output(&self) -> SoundSampleType {
        if self.length_counter == 0 || (self.shift_register & 1) != 0 {
            0
        } else if self.const_volume_enabled {
            self.const_volume as SoundSampleType
        } else {
            self.envelope_volume as SoundSampleType
        }
    }
}

impl APUNoise {
    const MAX_VOLUME: u8 = 0x0F;

    const NOISE_PERIODS: [u16; 16] = [
        4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
    ];

    pub fn new() -> Self {
        Self {
            const_volume: 0,
            envelope_volume: 0,
            const_volume_enabled: false,
            loop_enabled: false,
            length_counter: 0,
            short_mode: false,
            timer: 0,

            // The shift register should be initialized to 1.
            shift_register: 1,
            timer_counter: 0,

            envelope_period: 0,
            envelope_counter: 0,
            envelope_reload: false,
        }
    }
}
