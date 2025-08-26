use spin::{Lazy, RwLock};

pub struct NESCPU {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u16,
    pub reg_p: u16,
}

pub const CARRY_FLG: usize = 0;
pub const ZERO_FLG: usize = 1;
pub const INT_FLG: usize = 2;
pub const DECIMAL_FLG: usize = 3;
pub const BRK_FLG: usize = 4;
pub const OVERFLOW_FLAG: usize = 6;
pub const NEG_FLAG: usize = 7;

pub static NES_CPU: Lazy<RwLock<NESCPU>> = Lazy::new(|| {
    RwLock::new(NESCPU {
        reg_a: 0,
        reg_x: 0,
        reg_y: 0,
        reg_pc: 0,
        reg_sp: 0,
        reg_p: 0,
    })
});
