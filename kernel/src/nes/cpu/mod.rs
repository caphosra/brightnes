use alloc::format;
use bitflags::bitflags;
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
    pub reg_p: StatusFlags,
    pub cycles: u64,
    pub inst: u64,

    interrupt: InterruptType,
    #[serde(skip)]
    defer_irq: bool,
    stall_cycles: u32,
    ram: RAM,
    history: Vec<Option<Instruction>, { CPU::HISTORY_SIZE }>,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusFlags: u8 {
        const CARRY = 0b0000_0001;
        const ZERO = 0b0000_0010;
        const INT = 0b0000_0100;
        const DECIMAL = 0b0000_1000;
        const BRK = 0b0001_0000;
        const ONE = 0b0010_0000;
        const OVERFLOW = 0b0100_0000;
        const NEG = 0b1000_0000;
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InterruptType: u32 {
        const NMI = 0b0001;
        const BRK = 0b0010;
        const IRQ = 0b0100;
        const RST = 0b1000;
    }
}

static CPU_PTR: Lazy<Once<usize>> = Lazy::new(Once::new);

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
        self.interrupt = InterruptType::empty();
        self.defer_irq = false;
        self.ram = RAM::new();
        self.history = Vec::from_array([None; CPU::HISTORY_SIZE]);
    }

    pub fn dma_stall(&mut self) {
        self.stall_cycles += OAM_DMA_CYCLES;
    }

    pub fn dmc_stall(&mut self) {
        self.stall_cycles += 4;
    }

    fn do_dma_transfer(&mut self, ppu: &mut PPU, apu: &mut APU, cartridge: &mut Cartridge) {
        let mut data = [9; 0x100];
        for (i, d) in data.iter_mut().enumerate() {
            let addr = ppu.oam.dma_request_addr + i as u16;
            *d = CPUBus::read(addr, self, ppu, apu, cartridge);
        }
        for (i, d) in data.iter().enumerate() {
            ppu.oam.write(i as u8, *d);
        }
    }

    pub fn interrupt(&mut self, int_type: InterruptType) {
        self.interrupt.insert(int_type);
    }

    pub fn cancel_interrupt(&mut self, int_type: InterruptType) {
        self.interrupt.remove(int_type);
    }

    fn exec_pending_interrupt(
        &mut self,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> bool {
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

        if self.interrupt.contains(InterruptType::RST) {
            // RST

            self.interrupt.remove(InterruptType::RST);

            let lo = read_cpu_bus!(0xFFFC);
            let hi = read_cpu_bus!(0xFFFD);
            self.reg_sp = 0xFD;
            self.reg_pc = u16::from_le_bytes([lo, hi]);
            self.reg_p = StatusFlags::INT | StatusFlags::ONE;

            true
        } else if self.interrupt.contains(InterruptType::NMI) {
            // NMI

            self.interrupt.remove(InterruptType::NMI);

            self.reg_p.remove(StatusFlags::BRK);

            push_stack!((self.reg_pc >> 8) as u8);
            push_stack!((self.reg_pc & 0x00FF) as u8);
            push_stack!(self.reg_p.bits());

            let lo = read_cpu_bus!(0xFFFA);
            let hi = read_cpu_bus!(0xFFFB);
            self.reg_pc = u16::from_le_bytes([lo, hi]);

            self.reg_p.insert(StatusFlags::INT);

            true
        } else if self.interrupt.contains(InterruptType::BRK) {
            // BRK

            if self.reg_p.contains(StatusFlags::INT) {
                // Nested interrupt is not allowed for BRK.
                false
            } else {
                self.interrupt.remove(InterruptType::BRK);

                self.reg_p.insert(StatusFlags::BRK);

                push_stack!((self.reg_pc >> 8) as u8);
                push_stack!((self.reg_pc & 0x00FF) as u8);
                push_stack!(self.reg_p.bits());

                let lo = read_cpu_bus!(0xFFFE);
                let hi = read_cpu_bus!(0xFFFF);
                self.reg_pc = u16::from_le_bytes([lo, hi]);

                self.reg_p.remove(StatusFlags::BRK);
                self.reg_p.insert(StatusFlags::INT);

                true
            }
        } else if self.interrupt.contains(InterruptType::IRQ) {
            // IRQ

            if self.reg_p.contains(StatusFlags::INT) {
                // Nested interrupt is not allowed for IRQ.
                false
            } else {
                self.interrupt.remove(InterruptType::IRQ);

                self.reg_p.remove(StatusFlags::BRK);

                push_stack!((self.reg_pc >> 8) as u8);
                push_stack!((self.reg_pc & 0x00FF) as u8);
                push_stack!(self.reg_p.bits());

                let lo = read_cpu_bus!(0xFFFE);
                let hi = read_cpu_bus!(0xFFFF);
                self.reg_pc = u16::from_le_bytes([lo, hi]);

                self.reg_p.insert(StatusFlags::INT);

                true
            }
        } else {
            // No interrupt pending.
            false
        }
    }

    pub fn resolve_stall(
        &mut self,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> Option<u32> {
        if self.stall_cycles > Self::MAX_STALL_CYCLES {
            // The CPU is stalling for external reasons.
            self.stall_cycles -= Self::MAX_STALL_CYCLES;
            Some(Self::MAX_STALL_CYCLES)
        } else if self.stall_cycles > 0 {
            // DMA transfer ends.
            let cycles = self.stall_cycles;
            self.stall_cycles = 0;

            self.do_dma_transfer(ppu, apu, cartridge);

            Some(cycles)
        } else {
            None
        }
    }

    pub fn resolve_interrupt(
        &mut self,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> Option<u32> {
        // Check whether APU frame IRQ flag is set.
        if apu.frame_irq() {
            self.interrupt.insert(InterruptType::IRQ);
        }

        let suppress_irq = self.defer_irq;
        self.defer_irq = false;

        if suppress_irq && self.interrupt.contains(InterruptType::IRQ) {
            self.interrupt.remove(InterruptType::IRQ);
            let interrupted = self.exec_pending_interrupt(ppu, apu, cartridge);
            self.interrupt.insert(InterruptType::IRQ);
            return interrupted.then_some(Self::INTERRUPT_CYCLES);
        }

        // Check whether there is any pending interrupt.
        if self.exec_pending_interrupt(ppu, apu, cartridge) {
            Some(Self::INTERRUPT_CYCLES)
        } else {
            None
        }
    }

    pub fn fetch_instr(
        &mut self,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> Instruction {
        cartridge.begin_cpu_instruction();
        let instr = Instruction::fetch(self.reg_pc, self, ppu, apu, cartridge);

        // Record instruction to history.
        self.update_history(&instr);

        instr
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

    pub fn exec_instr(
        &mut self,
        instr: &Instruction,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> u32 {
        macro_rules! instr_resolve {
            () => {
                instr.addr_mode.resolve(self, ppu, apu, cartridge)
            };
        }

        macro_rules! instr_resolve_addr {
            () => {
                instr
                    .addr_mode
                    .resolve_addr(self, ppu, apu, cartridge)
                    .unwrap()
            };
        }

        macro_rules! instr_write {
            ($data:expr) => {
                instr.addr_mode.write($data, self, ppu, apu, cartridge)
            };
        }

        macro_rules! instr_rmw_write {
            ($before:expr, $after:expr) => {
                instr
                    .addr_mode
                    .write_read_modify($before, $after, self, ppu, apu, cartridge)
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

        let additional_cycles = match instr.instr_type {
            InstrType::ADC => {
                let (mem, additional_cycle) = instr_resolve!();
                let (sum, carry1) = mem.overflowing_add(self.reg_a);
                let (sum, carry2) =
                    sum.overflowing_add(self.reg_p.contains(StatusFlags::CARRY) as u8);

                // Calculate overflow
                let ans = (mem as i8) as i16
                    + (self.reg_a as i8) as i16
                    + self.reg_p.contains(StatusFlags::CARRY) as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));

                self.reg_p.set(StatusFlags::CARRY, carry1 || carry2);
                self.reg_p.set(StatusFlags::ZERO, sum == 0);
                self.reg_p.set(StatusFlags::NEG, sum & 0x80 != 0);

                self.reg_a = sum;
                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::AND => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a &= mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::ASL => {
                let (val, _) = match instr.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => instr_resolve!(),
                };
                let result = val << 1;

                self.reg_p.set(StatusFlags::CARRY, val & 0x80 != 0);
                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                match instr.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        instr_rmw_write!(val, result);
                    }
                };

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::BCC => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if !self.reg_p.contains(StatusFlags::CARRY) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BCS => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if self.reg_p.contains(StatusFlags::CARRY) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BEQ => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if self.reg_p.contains(StatusFlags::ZERO) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BIT => {
                let (mem, _) = instr_resolve!();

                self.reg_p.set(StatusFlags::NEG, mem & 0x80 != 0);
                self.reg_p.set(StatusFlags::OVERFLOW, mem & 0x40 != 0);
                self.reg_p.set(StatusFlags::ZERO, (self.reg_a & mem) == 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::BMI => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if self.reg_p.contains(StatusFlags::NEG) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BNE => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if !self.reg_p.contains(StatusFlags::ZERO) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BPL => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if !self.reg_p.contains(StatusFlags::NEG) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BRK => {
                self.interrupt(InterruptType::BRK);

                self.reg_pc += 2;
                0
            }
            InstrType::BVC => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if !self.reg_p.contains(StatusFlags::OVERFLOW) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::BVS => {
                let (addr, additional_cycle) = instr_resolve_addr!();
                if self.reg_p.contains(StatusFlags::OVERFLOW) {
                    self.reg_pc = addr;
                    additional_cycle + 1
                } else {
                    self.reg_pc += instr.addr_mode.size();
                    0
                }
            }
            InstrType::CLC => {
                self.reg_p.remove(StatusFlags::CARRY);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::CLD => {
                self.reg_p.remove(StatusFlags::DECIMAL);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::CLI => {
                if self.reg_p.contains(StatusFlags::INT) {
                    self.defer_irq = true;
                }
                self.reg_p.remove(StatusFlags::INT);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::CLV => {
                self.reg_p.remove(StatusFlags::OVERFLOW);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::CMP => {
                let (mem, additional_cycle) = instr_resolve!();
                let (res, borrow) = self.reg_a.overflowing_sub(mem);

                self.reg_p.set(StatusFlags::CARRY, !borrow);
                self.reg_p.set(StatusFlags::ZERO, res == 0);
                self.reg_p.set(StatusFlags::NEG, res & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::CPX => {
                let (mem, _) = instr_resolve!();
                let (res, borrow) = self.reg_x.overflowing_sub(mem);

                self.reg_p.set(StatusFlags::CARRY, !borrow);
                self.reg_p.set(StatusFlags::ZERO, res == 0);
                self.reg_p.set(StatusFlags::NEG, res & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::CPY => {
                let (mem, _) = instr_resolve!();
                let (res, borrow) = self.reg_y.overflowing_sub(mem);

                self.reg_p.set(StatusFlags::CARRY, !borrow);
                self.reg_p.set(StatusFlags::ZERO, res == 0);
                self.reg_p.set(StatusFlags::NEG, res & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::DEC => {
                let (mem, _) = instr_resolve!();

                let result = mem.wrapping_sub(1);

                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                instr_rmw_write!(mem, result);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::DEX => {
                self.reg_x = self.reg_x.wrapping_sub(1);

                self.reg_p.set(StatusFlags::ZERO, self.reg_x == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_x & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::DEY => {
                self.reg_y = self.reg_y.wrapping_sub(1);

                self.reg_p.set(StatusFlags::ZERO, self.reg_y == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_y & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::EOR => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a ^= mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::INC => {
                let (mem, _) = instr_resolve!();
                let result = mem.wrapping_add(1);

                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                instr_rmw_write!(mem, result);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::INX => {
                self.reg_x = self.reg_x.wrapping_add(1);

                self.reg_p.set(StatusFlags::ZERO, self.reg_x == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_x & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::INY => {
                self.reg_y = self.reg_y.wrapping_add(1);

                self.reg_p.set(StatusFlags::ZERO, self.reg_y == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_y & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::JMP => match instr.addr_mode {
                AddrMode::Absolute(addr) => {
                    self.reg_pc = addr;
                    0
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
                    0
                }
                _ => {
                    critical!(CPU, "Illegal JMP at PC={:#06x}", self.reg_pc);
                }
            },
            InstrType::JSR => match instr.addr_mode {
                AddrMode::Absolute(addr) => {
                    let return_addr = self.reg_pc + 2;
                    push_stack!((return_addr >> 8) as u8);
                    push_stack!((return_addr & 0x00FF) as u8);

                    self.reg_pc = addr;
                    0
                }
                _ => {
                    critical!(CPU, "Illegal JSR at PC={:#06x}", self.reg_pc);
                }
            },
            InstrType::LDA => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a = mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::LDX => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_x = mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_x == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_x & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::LDY => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_y = mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_y == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_y & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::LSR => {
                let (val, _) = match instr.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => instr_resolve!(),
                };
                let result = val >> 1;

                self.reg_p.set(StatusFlags::CARRY, val & 0x01 != 0);
                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                match instr.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        instr_rmw_write!(val, result);
                    }
                };

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::NOP => {
                let (_, additional_cycle) = instr_resolve!();

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::ORA => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a |= mem;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::PHA => {
                push_stack!(self.reg_a);
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::PHP => {
                push_stack!((self.reg_p | StatusFlags::BRK | StatusFlags::ONE).bits());
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::PLA => {
                self.reg_a = pop_stack!();

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::PLP => {
                let prev_interrupt_disabled = self.reg_p.contains(StatusFlags::INT);

                self.reg_p = StatusFlags::from_bits_retain(pop_stack!());
                self.reg_p.insert(StatusFlags::ONE);
                self.reg_p.remove(StatusFlags::BRK);

                if prev_interrupt_disabled && !self.reg_p.contains(StatusFlags::INT) {
                    self.defer_irq = true;
                }

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ROL => {
                let (val, _) = match instr.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => instr_resolve!(),
                };
                let carry = self.reg_p.contains(StatusFlags::CARRY) as u8;
                let result = (val << 1) | carry;

                self.reg_p.set(StatusFlags::CARRY, val & 0x80 != 0);
                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                match instr.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        instr_rmw_write!(val, result);
                    }
                };

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ROR => {
                let (val, _) = match instr.addr_mode {
                    AddrMode::Implied => (self.reg_a, 0),
                    _ => instr_resolve!(),
                };
                let carry = (self.reg_p.contains(StatusFlags::CARRY) as u8) << 7;
                let result = (val >> 1) | carry;

                self.reg_p.set(StatusFlags::CARRY, val & 0x01 != 0);
                self.reg_p.set(StatusFlags::ZERO, result == 0);
                self.reg_p.set(StatusFlags::NEG, result & 0x80 != 0);

                match instr.addr_mode {
                    AddrMode::Implied => {
                        self.reg_a = result;
                    }
                    _ => {
                        instr_rmw_write!(val, result);
                    }
                };

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::RTI => {
                let prev_interrupt_disabled = self.reg_p.contains(StatusFlags::INT);

                self.reg_p = StatusFlags::from_bits_retain(pop_stack!());
                self.reg_p.insert(StatusFlags::ONE);
                self.reg_p.remove(StatusFlags::BRK);

                if prev_interrupt_disabled && !self.reg_p.contains(StatusFlags::INT) {
                    self.defer_irq = true;
                }

                let lo = pop_stack!();
                let hi = pop_stack!();

                self.reg_pc = u16::from_le_bytes([lo, hi]);
                0
            }
            InstrType::RTS => {
                let lo = pop_stack!();
                let hi = pop_stack!();

                self.reg_pc = u16::from_le_bytes([lo, hi]) + 1;
                0
            }
            InstrType::SBC => {
                let (mem, additional_cycle) = instr_resolve!();
                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) =
                    diff.overflowing_sub(1 - self.reg_p.contains(StatusFlags::CARRY) as u8);

                // Calculate overflow
                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.reg_p.contains(StatusFlags::CARRY) as u8) as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));

                self.reg_p.set(StatusFlags::CARRY, !(borrow1 || borrow2));
                self.reg_p.set(StatusFlags::ZERO, diff == 0);
                self.reg_p.set(StatusFlags::NEG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::SEC => {
                self.reg_p.insert(StatusFlags::CARRY);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SED => {
                self.reg_p.insert(StatusFlags::DECIMAL);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SEI => {
                self.reg_p.insert(StatusFlags::INT);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::STA => {
                instr_write!(self.reg_a);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::STX => {
                instr_write!(self.reg_x);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::STY => {
                instr_write!(self.reg_y);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TAX => {
                self.reg_x = self.reg_a;

                self.reg_p.set(StatusFlags::ZERO, self.reg_x == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_x & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TAY => {
                self.reg_y = self.reg_a;

                self.reg_p.set(StatusFlags::ZERO, self.reg_y == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_y & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TSX => {
                self.reg_x = self.reg_sp;

                self.reg_p.set(StatusFlags::ZERO, self.reg_x == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_x & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TXA => {
                self.reg_a = self.reg_x;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TXS => {
                self.reg_sp = self.reg_x;

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TYA => {
                self.reg_a = self.reg_y;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ALR => {
                let (mem, _) = instr_resolve!();

                let carry = (self.reg_a & mem) & 1;
                self.reg_a = (self.reg_a & mem) >> 1;

                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);
                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::CARRY, carry != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ANC => {
                let (mem, _) = instr_resolve!();

                let carry = self.reg_a & 0x80;
                self.reg_a &= mem;

                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);
                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::CARRY, carry != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ANE => {
                critical!(CPU, "ANE is highly unstable.");
            }
            InstrType::ARR => {
                let (mem, _) = instr_resolve!();

                let ans = (self.reg_a & mem) as i16 + mem as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));

                let carry = (self.reg_a & mem) & 1;
                self.reg_a = ((self.reg_a & mem) >> 1) & (carry << 7);

                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);
                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                // I'm not sure
                self.reg_p.set(StatusFlags::CARRY, carry != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::DCP => {
                let (mem, _) = instr_resolve!();
                let mem = mem.wrapping_add_signed(-1);
                instr_rmw_write!(mem.wrapping_add(1), mem);

                let (res, borrow) = self.reg_a.overflowing_sub(mem);

                self.reg_p.set(StatusFlags::CARRY, !borrow);
                self.reg_p.set(StatusFlags::ZERO, res == 0);
                self.reg_p.set(StatusFlags::NEG, res & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::ISC => {
                let (mem, _) = instr_resolve!();
                let mem = mem.wrapping_add(1);
                instr_rmw_write!(mem.wrapping_sub(1), mem);

                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) =
                    diff.overflowing_sub(1 - self.reg_p.contains(StatusFlags::CARRY) as u8);

                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.reg_p.contains(StatusFlags::CARRY) as u8) as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));

                self.reg_p.set(StatusFlags::CARRY, !(borrow1 || borrow2));
                self.reg_p.set(StatusFlags::ZERO, diff == 0);
                self.reg_p.set(StatusFlags::NEG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::LAS => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a = mem & self.reg_sp;
                self.reg_x = self.reg_a;
                self.reg_sp = self.reg_a;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::LAX => {
                let (mem, additional_cycle) = instr_resolve!();
                self.reg_a = mem;
                self.reg_x = self.reg_a;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::LXA => {
                critical!(CPU, "LXA is highly unstable.");
            }
            InstrType::RLA => {
                let (val, _) = instr_resolve!();
                let carry = self.reg_p.contains(StatusFlags::CARRY) as u8;
                let result = (val << 1) | carry;

                instr_rmw_write!(val, result);

                self.reg_p.set(StatusFlags::CARRY, val & 0x80 != 0);

                self.reg_a &= result;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::RRA => {
                let (val, _) = instr_resolve!();
                let carry = self.reg_p.contains(StatusFlags::CARRY) as u8;
                let result = (val >> 1) | (carry << 7);

                instr_rmw_write!(val, result);

                let carry = val & 0x1;
                let (sum, carry1) = result.overflowing_add(self.reg_a);
                let (sum, carry2) = sum.overflowing_add(carry);

                // Calculate overflow
                let ans = (result as i8) as i16 + (self.reg_a as i8) as i16 + carry as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));
                self.reg_p.set(StatusFlags::CARRY, carry1 || carry2);
                self.reg_p.set(StatusFlags::ZERO, sum == 0);
                self.reg_p.set(StatusFlags::NEG, sum & 0x80 != 0);

                self.reg_a = sum;
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SAX => {
                let val = self.reg_a & self.reg_x;
                instr_write!(val);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SBX => {
                let val = self.reg_a & self.reg_x;
                let (mem, _) = instr_resolve!();
                let (res, borrow) = val.overflowing_sub(mem);

                self.reg_x = res;

                self.reg_p.set(StatusFlags::CARRY, !borrow);
                self.reg_p.set(StatusFlags::ZERO, res == 0);
                self.reg_p.set(StatusFlags::NEG, res & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SHA => {
                error!(CPU, "SHA is unstable.");
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SHX => {
                error!(CPU, "SHX is unstable.");
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SHY => {
                error!(CPU, "SHY is unstable.");
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SLO => {
                let (val, _) = instr_resolve!();
                let result = val << 1;

                instr_rmw_write!(val, result);

                self.reg_p.set(StatusFlags::CARRY, val & 0x80 != 0);

                self.reg_a |= result;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::SRE => {
                let (val, _) = instr_resolve!();
                let result = val >> 1;

                instr_rmw_write!(val, result);

                self.reg_p.set(StatusFlags::CARRY, val & 0x01 != 0);

                self.reg_a ^= result;

                self.reg_p.set(StatusFlags::ZERO, self.reg_a == 0);
                self.reg_p.set(StatusFlags::NEG, self.reg_a & 0x80 != 0);

                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::TAS => {
                error!(CPU, "TAS is unstable.");
                self.reg_pc += instr.addr_mode.size();
                0
            }
            InstrType::USBC => {
                let (mem, additional_cycle) = instr_resolve!();
                let (diff, borrow1) = self.reg_a.overflowing_sub(mem);
                let (diff, borrow2) =
                    diff.overflowing_sub(1 - self.reg_p.contains(StatusFlags::CARRY) as u8);

                // Calculate overflow
                let ans = (self.reg_a as i8) as i16
                    - (mem as i8) as i16
                    - (1 - self.reg_p.contains(StatusFlags::CARRY) as u8) as i16;
                self.reg_p
                    .set(StatusFlags::OVERFLOW, !(-0x80..=0x7F).contains(&ans));

                self.reg_p.set(StatusFlags::CARRY, !(borrow1 || borrow2));
                self.reg_p.set(StatusFlags::ZERO, diff == 0);
                self.reg_p.set(StatusFlags::NEG, diff & 0x80 != 0);

                self.reg_a = diff;
                self.reg_pc += instr.addr_mode.size();
                additional_cycle
            }
            InstrType::JAM => {
                critical!(CPU, "JAM encountered at PC={:#06X}", self.reg_pc);
            }
        };

        self.inst += 1;
        self.cycles += (instr.cycles + additional_cycles) as u64;

        additional_cycles as u32
    }

    fn update_history(&mut self, inst: &Instruction) {
        for i in 0..(Self::HISTORY_SIZE - 1) {
            self.history[i] = self.history[i + 1];
        }

        self.history[Self::HISTORY_SIZE - 1] = Some(*inst);

        self.send_inst_log(inst);
    }

    #[cfg(feature = "trace-cpu")]
    pub fn send_inst_log(&self, inst: &Instruction) {
        use crate::serial::Serial;

        Serial::communicate(|handler| {
            handler.write(
                format!(
                    "${:04X}: {:30} A={:02X} X={:02X} Y={:02X} P={:02X} SP={:02X}\n",
                    inst.pc, inst, self.reg_a, self.reg_x, self.reg_y, self.reg_p, self.reg_sp,
                )
                .as_bytes(),
            );
        });
    }

    #[cfg(not(feature = "trace-cpu"))]
    pub fn send_inst_log(&self, _inst: &Instruction) {}

    pub fn history_summary<F>(&self, mut handler: F)
    where
        F: FnMut(&str),
    {
        for i in 0..Self::HISTORY_SIZE {
            match self.history[i] {
                Some(inst) => {
                    if i == Self::HISTORY_SIZE - 1 {
                        handler(&format!("--> {:#06X}: {}", inst.pc, inst));
                    } else {
                        handler(&format!("    {:#06X}: {}", inst.pc, inst));
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
