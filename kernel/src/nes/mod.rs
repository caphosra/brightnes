use spin::{Lazy, RwLock};

pub struct NESConfig {
    #[allow(dead_code)]
    pub mapper: u8,
    pub mirroring: Mirroring,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

pub static NES_CONFIG: Lazy<RwLock<NESConfig>> = Lazy::new(|| {
    RwLock::new(NESConfig {
        mapper: 0,
        mirroring: Mirroring::Horizontal,
    })
});

impl From<u8> for Mirroring {
    fn from(value: u8) -> Self {
        if value == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }
}

pub mod bus;
pub mod cpu;
pub mod instr;
pub mod pad;
pub mod ppu;
pub mod ram;
pub mod rom;
