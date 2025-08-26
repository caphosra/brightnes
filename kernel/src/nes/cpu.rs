use spin::{Lazy, RwLock};

use crate::log;
use crate::nes::instr::{AddrMode, InstrType, Instruction};
use crate::nes::rom::NES_ROM;

pub struct NESCPU {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u16,
    pub reg_p: u16,
    pub stall_cycles: u8,
}

pub const CARRY_FLAG: usize = 0;
pub const ZERO_FLAG: usize = 1;
pub const INT_FLAG: usize = 2;
pub const DECIMAL_FLAG: usize = 3;
pub const BRK_FLAG: usize = 4;
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
        stall_cycles: 0,
    })
});

impl NESCPU {
    pub fn clock(&mut self) {
        if self.stall_cycles > 0 {
            self.stall_cycles -= 1;
        }
        if self.stall_cycles == 0 {
            self.execute();
        }
    }

    pub fn get_flag(&self, flag: usize) -> u8 {
        (self.reg_p & (1 << flag)) as u8
    }

    pub fn set_flag(&mut self, flag: usize, enabled: bool) {
        if enabled {
            self.reg_p |= 1 << flag;
        } else {
            self.reg_p &= !(1 << flag);
        }
    }

    pub fn execute(&mut self) {
        let rom = NES_ROM.get().unwrap();
        let (_, code) = rom.prg_rom.split_at(self.reg_pc as usize);
        let inst = Instruction::fetch(code);

        let cycles = match inst.instr_type {
            InstrType::ADC => {
                let (mem, additional_cycle) = inst.addr_mode.resolve(self);
                let (sum, carry1) = mem.overflowing_add(self.reg_a);
                let (sum, carry2) = sum.overflowing_add(self.get_flag(CARRY_FLAG));

                // Calculate overflow
                let ans = mem as i16 + sum as i16 + self.get_flag(CARRY_FLAG) as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0xFF || ans < 0);

                self.set_flag(CARRY_FLAG, carry1 || carry2);
                self.set_flag(ZERO_FLAG, sum == 0);
                self.set_flag(NEG_FLAG, sum & 0x80 != 0);

                self.reg_a = sum;
                inst.cycles + additional_cycle
            }
            InstrType::AND => {
                let (mem, additional_cycle) = inst.addr_mode.resolve(self);
                self.reg_a &= mem;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);
                inst.cycles + additional_cycle
            }
            InstrType::ASL => {
                let (val, _) = match inst.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => inst.addr_mode.resolve(self),
                };
                let result = val << 1;

                self.set_flag(CARRY_FLAG, val & 0x80 != 0);
                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                match inst.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        inst.addr_mode.write(self, result);
                    }
                };

                // ASL does not have additional cycle on page crossing
                inst.cycles
            }
            _ => {
                log!("[CPU] Unimplemented instruction at PC={:#06x}", self.reg_pc);
                0
            }
        };
    }
}
