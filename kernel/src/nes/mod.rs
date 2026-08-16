use alloc::string::ToString;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
}

impl From<u8> for Mirroring {
    fn from(value: u8) -> Self {
        if value == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }
}

impl ToString for Mirroring {
    fn to_string(&self) -> alloc::string::String {
        match self {
            Mirroring::Horizontal => "Horizontal".to_string(),
            Mirroring::Vertical => "Vertical".to_string(),
            Mirroring::SingleScreenLower => "Single Screen Lower".to_string(),
            Mirroring::SingleScreenUpper => "Single Screen Upper".to_string(),
        }
    }
}

pub mod apu;
pub mod cartridge;
pub mod cpu;
pub mod pad;
pub mod ppu;
