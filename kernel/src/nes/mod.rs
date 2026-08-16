use core::fmt::Display;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Mirroring {
    Horizontal,
    Vertical,
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
        }
    }
}

pub mod apu;
pub mod cartridge;
pub mod cpu;
pub mod pad;
pub mod ppu;
