use alloc::format;
use alloc::string::{String, ToString};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use spin::{Lazy, Once};

use crate::mem::MemoryAllocator;
use crate::nes::apu::APU;
use crate::nes::cartridge::Cartridge;
use crate::nes::cpu::bus::CPUBus;
use crate::nes::cpu::instr::{AddrMode, InstrType, Instruction};
use crate::nes::cpu::ram::RAM;
use crate::nes::ppu::oam::OAM_DMA_CYCLES;
use crate::nes::ppu::PPU;
use crate::{critical, error};

#[derive(Serialize, Deserialize)]
pub struct CPU {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u8,
    pub reg_p: u8,
    pub cycles: u64,
    pub inst: u64,

    interrupt: Option<InterruptType>,
    stall_cycles: u32,
    ram: RAM,
    history: Vec<Option<Instruction>, { CPU::HISTORY_SIZE }>,
}

pub const CARRY_FLAG: usize = 0;
pub const ZERO_FLAG: usize = 1;
pub const INT_FLAG: usize = 2;
pub const DECIMAL_FLAG: usize = 3;
pub const BRK_FLAG: usize = 4;
pub const ONE_FLAG: usize = 5;
pub const OVERFLOW_FLAG: usize = 6;
pub const NEG_FLAG: usize = 7;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptType {
    NMI,
    BRK,
    IRQ,
    RST,
}

impl ToString for InterruptType {
    fn to_string(&self) -> String {
        match self {
            InterruptType::NMI => "NMI".to_string(),
            InterruptType::BRK => "BRK".to_string(),
            InterruptType::IRQ => "IRQ".to_string(),
            InterruptType::RST => "RST".to_string(),
        }
    }
}

static CPU_PTR: Lazy<Once<usize>> = Lazy::new(|| Once::new());

impl CPU {
    pub const CLOCK_FREQ: u32 = 1789773;

    pub const HISTORY_SIZE: usize = 16;
    const MAX_STALL_CYCLES: u32 = 8;
    const INTERRUPT_CYCLES: u32 = 7;

    pub fn get() -> &'static mut CPU {
        let cpu_raw_ptr = *CPU_PTR.call_once(|| {
            // Allocate memory for CPU.
            let cpu_raw_ptr = MemoryAllocator::alloc_zeroed::<CPU>();
            cpu_raw_ptr as usize
        }) as *mut CPU;

        unsafe { cpu_raw_ptr.as_mut() }.unwrap()
    }

    pub fn init(&mut self) {
        self.reg_p = 0x24;
        self.interrupt = None;
        self.ram = RAM::new();
        self.history = Vec::from_array([None; CPU::HISTORY_SIZE]);
    }

    pub fn dma_stall(&mut self) {
        self.stall_cycles += OAM_DMA_CYCLES;
    }

    fn do_dma_transfer(&mut self, ppu: &mut PPU, apu: &mut APU, cartridge: &mut Cartridge) {
        let mut data = [9; 0x100];
        for i in 0..=0xFF {
            let addr = ppu.oam.dma_request_addr + i as u16;
            data[i] = CPUBus::read(addr, self, ppu, apu, cartridge);
        }
        for i in 0..=0xFF {
            ppu.oam.write(i as u8, data[i]);
        }
    }

    pub fn interrupt(&mut self, int_type: InterruptType) {
        match self.interrupt {
            Some(int_type) => {
                error!(
                    CPU,
                    "Interrupt request is already pending: {}",
                    int_type.to_string()
                );
            }
            None => {
                self.interrupt = Some(int_type);
            }
        }
    }

    fn exec_interrupt(
        &mut self,
        int_type: InterruptType,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) {
        if self.get_flag(INT_FLAG) != 0
            && (int_type == InterruptType::BRK)
            && (int_type == InterruptType::IRQ)
        {
            // Nested interrupt is not allowed for BRK and IRQ.
            return;
        }

        self.set_flag(INT_FLAG, true);

        macro_rules! push_stack {
            ($data:expr) => {
                self.push_stack($data, ppu, apu, cartridge)
            };
        }

        macro_rules! read_cpu_bus {
            ($addr:expr) => {
                CPUBus::read($addr, self, ppu, apu, cartridge)
            };
        }

        match int_type {
            InterruptType::BRK => {
                self.reg_pc += 1;
                push_stack!((self.reg_pc >> 8) as u8);
                push_stack!((self.reg_pc & 0x00FF) as u8);
                push_stack!(self.reg_p | 0b110000);

                let lo = read_cpu_bus!(0xFFFE);
                let hi = read_cpu_bus!(0xFFFF);
                self.reg_pc = u16::from_le_bytes([lo, hi]);
            }
            InterruptType::NMI => {
                self.set_flag(BRK_FLAG, false);

                push_stack!((self.reg_pc >> 8) as u8);
                push_stack!((self.reg_pc & 0x00FF) as u8);

                push_stack!((self.reg_p & 0b11001111) | 1 << 5);

                let lo = read_cpu_bus!(0xFFFA);
                let hi = read_cpu_bus!(0xFFFB);
                self.reg_pc = u16::from_le_bytes([lo, hi]);
            }
            InterruptType::IRQ => {
                self.set_flag(BRK_FLAG, false);

                push_stack!((self.reg_pc >> 8) as u8);
                push_stack!((self.reg_pc & 0x00FF) as u8);

                push_stack!((self.reg_p & 0b11001111) | 1 << 5);

                let lo = read_cpu_bus!(0xFFFE);
                let hi = read_cpu_bus!(0xFFFF);
                self.reg_pc = u16::from_le_bytes([lo, hi]);
            }
            InterruptType::RST => {
                let lo = read_cpu_bus!(0xFFFC);
                let hi = read_cpu_bus!(0xFFFD);
                self.reg_sp = 0xFD;
                self.reg_pc = u16::from_le_bytes([lo, hi]);
            }
        }
    }

    pub fn clock(&mut self, ppu: &mut PPU, apu: &mut APU, cartridge: &mut Cartridge) -> u32 {
        if self.stall_cycles > Self::MAX_STALL_CYCLES {
            // The CPU is stalling for external reasons.
            self.stall_cycles -= Self::MAX_STALL_CYCLES;
            Self::MAX_STALL_CYCLES
        } else if self.stall_cycles > 0 {
            // DMA transfer ends.
            let cycles = self.stall_cycles;
            self.stall_cycles = 0;

            self.do_dma_transfer(ppu, apu, cartridge);

            cycles
        } else {
            match self.interrupt {
                Some(int_type) => {
                    self.exec_interrupt(int_type, ppu, apu, cartridge);
                    self.interrupt = None;

                    Self::INTERRUPT_CYCLES
                }
                None => {
                    // The CPU is not stalling so execute the next instruction.
                    let cycles = self.execute(ppu, apu, cartridge);
                    self.inst += 1;
                    self.cycles += cycles as u64;
                    cycles
                }
            }
        }
    }

    pub fn get_flag(&self, flag: usize) -> u8 {
        ((self.reg_p & (1 << flag)) >> flag) as u8
    }

    pub fn set_flag(&mut self, flag: usize, enabled: bool) {
        if enabled {
            self.reg_p |= 1 << flag;
        } else {
            self.reg_p &= !(1 << flag);
        }
    }

    pub fn push_stack(
        &mut self,
        data: u8,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) {
        let addr = 0x0100 | self.reg_sp as u16;
        CPUBus::write(addr, data, self, ppu, apu, cartridge);
        self.reg_sp = self.reg_sp.wrapping_sub(1);
    }

    pub fn pop_stack(&mut self, ppu: &mut PPU, apu: &mut APU, cartridge: &mut Cartridge) -> u8 {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        let addr = 0x0100 | self.reg_sp as u16;
        CPUBus::read(addr, self, ppu, apu, cartridge)
    }

    pub fn execute(&mut self, ppu: &mut PPU, apu: &mut APU, cartridge: &mut Cartridge) -> u32 {
        let inst = Instruction::fetch(self.reg_pc, self, ppu, apu, cartridge);

        // Record instruction to history.
        self.update_history(&inst);

        macro_rules! inst_resolve {
            () => {
                inst.addr_mode.resolve(self, ppu, apu, cartridge)
            };
        }

        macro_rules! inst_resolve_addr {
            () => {
                inst.addr_mode
                    .resolve_addr(self, ppu, apu, cartridge)
                    .unwrap()
            };
        }

        macro_rules! inst_write {
            ($data:expr) => {
                inst.addr_mode.write($data, self, ppu, apu, cartridge)
            };
        }

        macro_rules! push_stack {
            ($data:expr) => {
                self.push_stack($data, ppu, apu, cartridge)
            };
        }

        macro_rules! pop_stack {
            () => {
                self.pop_stack(ppu, apu, cartridge)
            };
        }

        let cycles = match inst.instr_type {
            InstrType::ADC => {
                let (mem, additional_cycle) = inst_resolve!();
                let (sum, carry1) = mem.overflowing_add(self.reg_a);
                let (sum, carry2) = sum.overflowing_add(self.get_flag(CARRY_FLAG));

                // Calculate overflow
                let ans = (mem as i8) as i16
                    + (self.reg_a as i8) as i16
                    + self.get_flag(CARRY_FLAG) as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);

                self.set_flag(CARRY_FLAG, carry1 || carry2);
                self.set_flag(ZERO_FLAG, sum == 0);
                self.set_flag(NEG_FLAG, sum & 0x80 != 0);

                self.reg_a = sum;
                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::AND => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a &= mem;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::ASL => {
                let (val, _) = match inst.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => inst_resolve!(),
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
                        inst_write!(result);
                    }
                };

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::BCC => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(CARRY_FLAG) == 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BCS => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(CARRY_FLAG) != 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BEQ => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(ZERO_FLAG) != 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BIT => {
                let (mem, _) = inst_resolve!();

                self.set_flag(NEG_FLAG, mem & 0x80 != 0);
                self.set_flag(OVERFLOW_FLAG, mem & 0x40 != 0);
                self.set_flag(ZERO_FLAG, (self.reg_a & mem) == 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::BMI => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(NEG_FLAG) != 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BNE => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(ZERO_FLAG) == 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BPL => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(NEG_FLAG) == 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BRK => {
                self.set_flag(BRK_FLAG, true);
                self.interrupt(InterruptType::BRK);

                inst.cycles - 1
            }
            InstrType::BVC => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(OVERFLOW_FLAG) == 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::BVS => {
                let (addr, additional_cycle) = inst_resolve_addr!();
                if self.get_flag(OVERFLOW_FLAG) != 0 {
                    self.reg_pc = addr;
                    inst.cycles + additional_cycle + 1
                } else {
                    self.reg_pc += inst.addr_mode.size();
                    inst.cycles
                }
            }
            InstrType::CLC => {
                self.set_flag(CARRY_FLAG, false);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::CLD => {
                self.set_flag(DECIMAL_FLAG, false);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::CLI => {
                self.set_flag(INT_FLAG, false);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::CLV => {
                self.set_flag(OVERFLOW_FLAG, false);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::CMP => {
                let (mem, additional_cycle) = inst_resolve!();
                let (res, borrow) = self.reg_a.overflowing_sub(mem);

                self.set_flag(CARRY_FLAG, !borrow);
                self.set_flag(ZERO_FLAG, res == 0);
                self.set_flag(NEG_FLAG, res & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::CPX => {
                let (mem, _) = inst_resolve!();
                let (res, borrow) = self.reg_x.overflowing_sub(mem);

                self.set_flag(CARRY_FLAG, !borrow);
                self.set_flag(ZERO_FLAG, res == 0);
                self.set_flag(NEG_FLAG, res & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::CPY => {
                let (mem, _) = inst_resolve!();
                let (res, borrow) = self.reg_y.overflowing_sub(mem);

                self.set_flag(CARRY_FLAG, !borrow);
                self.set_flag(ZERO_FLAG, res == 0);
                self.set_flag(NEG_FLAG, res & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::DEC => {
                let (mem, _) = inst_resolve!();

                let result = mem.wrapping_sub(1);

                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                inst_write!(result);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::DEX => {
                self.reg_x = self.reg_x.wrapping_sub(1);

                self.set_flag(ZERO_FLAG, self.reg_x == 0);
                self.set_flag(NEG_FLAG, self.reg_x & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::DEY => {
                self.reg_y = self.reg_y.wrapping_sub(1);

                self.set_flag(ZERO_FLAG, self.reg_y == 0);
                self.set_flag(NEG_FLAG, self.reg_y & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::EOR => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a ^= mem;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::INC => {
                let (mem, _) = inst_resolve!();
                let result = mem.wrapping_add(1);

                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                inst_write!(result);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::INX => {
                self.reg_x = self.reg_x.wrapping_add(1);

                self.set_flag(ZERO_FLAG, self.reg_x == 0);
                self.set_flag(NEG_FLAG, self.reg_x & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::INY => {
                self.reg_y = self.reg_y.wrapping_add(1);

                self.set_flag(ZERO_FLAG, self.reg_y == 0);
                self.set_flag(NEG_FLAG, self.reg_y & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::JMP => match inst.addr_mode {
                AddrMode::Absolute(addr) => {
                    self.reg_pc = addr;
                    inst.cycles
                }
                AddrMode::Indirect(addr) => {
                    let lo = CPUBus::read(addr, self, ppu, apu, cartridge);

                    // Since NES cannot reflect the carry in cycles, we should calculate it separately.
                    let hi = CPUBus::read(
                        (addr & 0xFF00) | (addr.wrapping_add(1) & 0x00FF),
                        self,
                        ppu,
                        apu,
                        cartridge,
                    );

                    self.reg_pc = u16::from_le_bytes([lo, hi]);
                    inst.cycles
                }
                _ => {
                    critical!(CPU, "Illegal JMP at PC={:#06x}", self.reg_pc);
                }
            },
            InstrType::JSR => match inst.addr_mode {
                AddrMode::Absolute(addr) => {
                    let return_addr = self.reg_pc + 2;
                    push_stack!((return_addr >> 8) as u8);
                    push_stack!((return_addr & 0x00FF) as u8);

                    self.reg_pc = addr;
                    inst.cycles
                }
                _ => {
                    critical!(CPU, "Illegal JSR at PC={:#06x}", self.reg_pc);
                }
            },
            InstrType::LDA => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a = mem;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::LDX => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_x = mem;

                self.set_flag(ZERO_FLAG, self.reg_x == 0);
                self.set_flag(NEG_FLAG, self.reg_x & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::LDY => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_y = mem;

                self.set_flag(ZERO_FLAG, self.reg_y == 0);
                self.set_flag(NEG_FLAG, self.reg_y & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::LSR => {
                let (val, _) = match inst.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => inst_resolve!(),
                };
                let result = val >> 1;

                self.set_flag(CARRY_FLAG, val & 0x01 != 0);
                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                match inst.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        inst_write!(result);
                    }
                };

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::NOP => {
                let (_, additional_cycle) = inst_resolve!();

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::ORA => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a |= mem;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::PHA => {
                push_stack!(self.reg_a);
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::PHP => {
                push_stack!(self.reg_p | 0b110000);
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::PLA => {
                self.reg_a = pop_stack!();

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::PLP => {
                self.reg_p = (pop_stack!() & !(1 << BRK_FLAG))
                    | (self.reg_p & (1 << BRK_FLAG))
                    | (1 << ONE_FLAG);
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ROL => {
                let (val, _) = match inst.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => inst_resolve!(),
                };
                let carry = self.get_flag(CARRY_FLAG);
                let result = (val << 1) | carry;

                self.set_flag(CARRY_FLAG, val & 0x80 != 0);
                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                match inst.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        inst_write!(result);
                    }
                };

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ROR => {
                let (val, _) = match inst.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => inst_resolve!(),
                };
                let carry = self.get_flag(CARRY_FLAG) << 7;
                let result = (val >> 1) | carry;

                self.set_flag(CARRY_FLAG, val & 0x01 != 0);
                self.set_flag(ZERO_FLAG, result == 0);
                self.set_flag(NEG_FLAG, result & 0x80 != 0);

                match inst.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        inst_write!(result);
                    }
                };

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::RTI => {
                self.reg_p = (pop_stack!() & !(1 << BRK_FLAG))
                    | (self.reg_p & (1 << BRK_FLAG))
                    | 1 << ONE_FLAG;

                let lo = pop_stack!();
                let hi = pop_stack!();

                self.reg_pc = u16::from_le_bytes([lo, hi]);
                inst.cycles
            }
            InstrType::RTS => {
                let lo = pop_stack!();
                let hi = pop_stack!();

                self.reg_pc = u16::from_le_bytes([lo, hi]) + 1;
                inst.cycles
            }
            InstrType::SBC => {
                let (mem, additional_cycle) = inst_resolve!();
                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) = diff.overflowing_sub(1 - self.get_flag(CARRY_FLAG));

                // Calculate overflow
                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.get_flag(CARRY_FLAG)) as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);

                self.set_flag(CARRY_FLAG, !(borrow1 || borrow2));
                self.set_flag(ZERO_FLAG, diff == 0);
                self.set_flag(NEG_FLAG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::SEC => {
                self.set_flag(CARRY_FLAG, true);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SED => {
                self.set_flag(DECIMAL_FLAG, true);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SEI => {
                self.set_flag(INT_FLAG, true);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::STA => {
                inst_write!(self.reg_a);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::STX => {
                inst_write!(self.reg_x);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::STY => {
                inst_write!(self.reg_y);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TAX => {
                self.reg_x = self.reg_a;

                self.set_flag(ZERO_FLAG, self.reg_x == 0);
                self.set_flag(NEG_FLAG, self.reg_x & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TAY => {
                self.reg_y = self.reg_a;

                self.set_flag(ZERO_FLAG, self.reg_y == 0);
                self.set_flag(NEG_FLAG, self.reg_y & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TSX => {
                self.reg_x = self.reg_sp;

                self.set_flag(ZERO_FLAG, self.reg_x == 0);
                self.set_flag(NEG_FLAG, self.reg_x & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TXA => {
                self.reg_a = self.reg_x;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TXS => {
                self.reg_sp = self.reg_x;

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TYA => {
                self.reg_a = self.reg_y;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ALR => {
                let (mem, _) = inst_resolve!();

                let carry = (self.reg_a & mem) & 1;
                self.reg_a = (self.reg_a & mem) >> 1;

                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);
                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(CARRY_FLAG, carry != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ANC => {
                let (mem, _) = inst_resolve!();

                let carry = self.reg_a & 0x80;
                self.reg_a = self.reg_a & mem;

                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);
                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(CARRY_FLAG, carry != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ANE => {
                critical!(CPU, "ANE is highly unstable.");
            }
            InstrType::ARR => {
                let (mem, _) = inst_resolve!();

                let ans = (self.reg_a & mem) as i16 + mem as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);

                let carry = (self.reg_a & mem) & 1;
                self.reg_a = ((self.reg_a & mem) >> 1) & (carry << 7);

                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);
                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                // I'm not sure
                self.set_flag(CARRY_FLAG, carry != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::DCP => {
                let (mem, _) = inst_resolve!();
                let mem = mem.wrapping_add_signed(-1);
                inst_write!(mem);

                let (res, borrow) = self.reg_a.overflowing_sub(mem);

                self.set_flag(CARRY_FLAG, !borrow);
                self.set_flag(ZERO_FLAG, res == 0);
                self.set_flag(NEG_FLAG, res & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::ISC => {
                let (mem, _) = inst_resolve!();
                let mem = mem.wrapping_add(1);
                inst_write!(mem);

                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) = diff.overflowing_sub(1 - self.get_flag(CARRY_FLAG));

                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.get_flag(CARRY_FLAG)) as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);

                self.set_flag(CARRY_FLAG, !(borrow1 || borrow2));
                self.set_flag(ZERO_FLAG, diff == 0);
                self.set_flag(NEG_FLAG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::LAS => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a = mem & self.reg_sp;
                self.reg_x = self.reg_a;
                self.reg_sp = self.reg_a;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::LAX => {
                let (mem, additional_cycle) = inst_resolve!();
                self.reg_a = mem;
                self.reg_x = self.reg_a;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::LXA => {
                critical!(CPU, "LXA is highly unstable.");
            }
            InstrType::RLA => {
                let (val, _) = inst_resolve!();
                let carry = self.get_flag(CARRY_FLAG);
                let result = (val << 1) | carry;

                inst_write!(result);

                self.set_flag(CARRY_FLAG, val & 0x80 != 0);

                self.reg_a = self.reg_a & result;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::RRA => {
                let (val, _) = inst_resolve!();
                let carry = self.get_flag(CARRY_FLAG);
                let result = (val >> 1) | (carry << 7);

                inst_write!(result);

                let carry = val & 0x1;
                let (sum, carry1) = result.overflowing_add(self.reg_a);
                let (sum, carry2) = sum.overflowing_add(carry);

                // Calculate overflow
                let ans = (result as i8) as i16 + (self.reg_a as i8) as i16 + carry as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);
                self.set_flag(CARRY_FLAG, carry1 || carry2);
                self.set_flag(ZERO_FLAG, sum == 0);
                self.set_flag(NEG_FLAG, sum & 0x80 != 0);

                self.reg_a = sum;
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SAX => {
                let val = self.reg_a & self.reg_x;
                inst_write!(val);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SBX => {
                let val = self.reg_a & self.reg_x;
                let (mem, _) = inst_resolve!();
                let (res, borrow) = val.overflowing_sub(mem);

                self.reg_x = res;

                self.set_flag(CARRY_FLAG, !borrow);
                self.set_flag(ZERO_FLAG, res == 0);
                self.set_flag(NEG_FLAG, res & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SHA => {
                error!(CPU, "SHA is unstable.");
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SHX => {
                error!(CPU, "SHX is unstable.");
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SHY => {
                error!(CPU, "SHY is unstable.");
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SLO => {
                let (val, _) = inst_resolve!();
                let result = val << 1;

                inst_write!(result);

                self.set_flag(CARRY_FLAG, val & 0x80 != 0);

                self.reg_a = self.reg_a | result;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::SRE => {
                let (val, _) = inst_resolve!();
                let result = val >> 1;

                inst_write!(result);

                self.set_flag(CARRY_FLAG, val & 0x01 != 0);

                self.reg_a = self.reg_a ^ result;

                self.set_flag(ZERO_FLAG, self.reg_a == 0);
                self.set_flag(NEG_FLAG, self.reg_a & 0x80 != 0);

                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::TAS => {
                error!(CPU, "TAS is unstable.");
                self.reg_pc += inst.addr_mode.size();
                inst.cycles
            }
            InstrType::USBC => {
                let (mem, additional_cycle) = inst_resolve!();
                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) = diff.overflowing_sub(1 - self.get_flag(CARRY_FLAG));

                // Calculate overflow
                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.get_flag(CARRY_FLAG)) as i16;
                self.set_flag(OVERFLOW_FLAG, ans > 0x7F || ans < -0x80);

                self.set_flag(CARRY_FLAG, !(borrow1 || borrow2));
                self.set_flag(ZERO_FLAG, diff == 0);
                self.set_flag(NEG_FLAG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += inst.addr_mode.size();
                inst.cycles + additional_cycle
            }
            InstrType::JAM => {
                critical!(CPU, "JAM encountered at PC={:#06X}", self.reg_pc);
            }
        };

        cycles as u32
    }

    fn update_history(&mut self, inst: &Instruction) {
        for i in 0..(Self::HISTORY_SIZE - 1) {
            self.history[i] = self.history[i + 1];
        }

        self.history[Self::HISTORY_SIZE - 1] = Some(inst.clone());
    }

    pub fn history_summary<F>(&self, mut handler: F)
    where
        F: FnMut(&str),
    {
        for i in 0..Self::HISTORY_SIZE {
            match self.history[i] {
                Some(inst) => {
                    if i == Self::HISTORY_SIZE - 1 {
                        handler(&format!("--> {:#06X}: {}", inst.pc, inst.to_string()));
                    } else {
                        handler(&format!("    {:#06X}: {}", inst.pc, inst.to_string()));
                    }
                }
                None => {
                    handler("    ------: ------");
                }
            }
        }
    }

    pub fn report_backtrace(&self) {
        error!(CPU, "Backtrace:");
        self.history_summary(|line| {
            error!(CPU, "{}", line);
        });
    }
}

pub mod bus;
pub mod instr;
pub mod ram;
