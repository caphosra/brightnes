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

pub mod cartridge;
pub mod cpu;
pub mod pad;
pub mod ppu;
