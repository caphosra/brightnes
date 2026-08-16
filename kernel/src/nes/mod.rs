use core::fmt::Display;

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

impl Display for Mirroring {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mirroring::Horizontal => write!(f, "Horizontal"),
            Mirroring::Vertical => write!(f, "Vertical"),
            Mirroring::SingleScreenLower => write!(f, "Single Screen Lower"),
            Mirroring::SingleScreenUpper => write!(f, "Single Screen Upper"),
        }
    }
}

pub mod apu;
pub mod cartridge;
pub mod cpu;
pub mod pad;
pub mod ppu;
