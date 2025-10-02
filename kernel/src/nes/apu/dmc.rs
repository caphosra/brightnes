use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DMC {
    pub active: bool,
    pub irq: bool,
}

impl DMC {
    pub fn new() -> Self {
        Self {
            active: false,
            irq: false,
        }
    }

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
        // TODO
    }
}
