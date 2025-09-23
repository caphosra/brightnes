use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct APUTriangle {
    pub active: bool,
}

impl APUTriangle {
    pub fn new() -> Self {
        Self { active: false }
    }

    pub fn write_reg(&mut self, _addr: u16, _data: u8) {
        // TODO
    }
}
