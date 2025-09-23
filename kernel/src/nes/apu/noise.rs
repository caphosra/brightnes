use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct APUNoise {
    pub active: bool,
}

impl APUNoise {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
        // TODO
    }
}
