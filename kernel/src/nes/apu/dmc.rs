use serde::{Deserialize, Serialize};

use crate::{critical, warn};

#[derive(Serialize, Deserialize)]
pub struct DMC {
    pub irq: bool,
    pub length_counter: u16,
    pub loop_enabled: bool,
    pub rate: u8,
    pub sample_address: u16,
    pub sample_length: u16,
}

impl DMC {
    pub fn new() -> Self {
        Self {
            irq: false,
            length_counter: 0,
            loop_enabled: false,
            rate: 0,
            sample_address: 0,
            sample_length: 0,
        }
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0 => {
                self.irq = ((data >> 7) & 1) != 0;
                self.loop_enabled = ((data >> 6) & 1) != 0;
                self.rate = data & 0b1111;

                if self.irq {
                    warn!(APU, "DMC IRQ enabled, which is not supported.");
                }
            }
            1 => {}
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
}
