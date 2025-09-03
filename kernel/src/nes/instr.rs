//
// 6502 Instruction Set
// https://www.masswerk.at/6502/6502_instruction_set.html
//

use crate::nes::bus::NESBus;
use crate::nes::cpu::NESCPU;

pub enum InstrType {
    // Transfer Instructions
    LDA,
    LDX,
    LDY,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    // Stack Instructions
    PHA,
    PHP,
    PLA,
    PLP,
    // Decrement and Increment Instructions
    DEC,
    DEX,
    DEY,
    INC,
    INX,
    INY,
    // Arithmetic Instructions
    ADC,
    SBC,
    // Logical Instructions
    AND,
    EOR,
    ORA,
    // Shift Instructions
    ASL,
    LSR,
    ROL,
    ROR,
    // Flag Instructions
    CLC,
    CLD,
    CLI,
    CLV,
    SEC,
    SED,
    SEI,
    // Comparison Instructions
    CMP,
    CPX,
    CPY,
    // Conditional Branch Instructions
    BCC,
    BCS,
    BEQ,
    BMI,
    BNE,
    BPL,
    BVC,
    BVS,
    // Jump and Subroutine Instructions
    JMP,
    JSR,
    RTS,
    // Interrupt Instructions
    BRK,
    RTI,
    // Other Instructions
    BIT,
    NOP,

    // Illegal Instructions
    ALR,
    ANC,
    ANE,
    ARR,

    // Illegal
    Invalid(u8),
}

pub enum AddrMode {
    Implied,
    Immediate(u8),
    Absolute(u16),
    ZeroPage(u8),
    AbsoluteX(u16),
    AbsoluteY(u16),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Indirect(u16),
    IndirectX(u8),
    IndirectY(u8),
    Relative(i8),
}

pub struct Instruction {
    pub instr_type: InstrType,
    pub addr_mode: AddrMode,
    pub cycles: u8,
}

impl Instruction {
    pub fn fetch(pc: u16) -> Self {
        macro_rules! instr {
            ($inst:tt, Implied, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::Implied,
                    cycles: $cycles,
                }
            };
            ($inst:tt, Immediate, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, Absolute, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                        NESBus::read(pc + 1),
                        NESBus::read(pc + 2),
                    ])),
                    cycles: $cycles,
                }
            };
            ($inst:tt, ZeroPage, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, AbsoluteX, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                        NESBus::read(pc + 1),
                        NESBus::read(pc + 2),
                    ])),
                    cycles: $cycles,
                }
            };
            ($inst:tt, AbsoluteY, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                        NESBus::read(pc + 1),
                        NESBus::read(pc + 2),
                    ])),
                    cycles: $cycles,
                }
            };
            ($inst:tt, ZeroPageX, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, ZeroPageY, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::ZeroPageY(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, Indirect, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::Indirect(u16::from_le_bytes([
                        NESBus::read(pc + 1),
                        NESBus::read(pc + 2),
                    ])),
                    cycles: $cycles,
                }
            };
            ($inst:tt, IndirectX, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, IndirectY, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                    cycles: $cycles,
                }
            };
            ($inst:tt, Relative, $cycles:expr) => {
                Instruction {
                    instr_type: InstrType::$inst,
                    addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                    cycles: $cycles,
                }
            };
        }

        match NESBus::read(pc) {
            0x00 => instr!(BRK, Implied, 7),
            0x01 => instr!(ORA, IndirectX, 6),
            0x05 => instr!(ORA, ZeroPage, 3),
            0x06 => instr!(ASL, ZeroPage, 5),
            0x08 => instr!(PHP, Implied, 3),
            0x09 => instr!(ORA, Immediate, 2),
            0x0A => instr!(ASL, Implied, 2),
            0x0B => instr!(ANC, Immediate, 2),
            0x0D => instr!(ORA, Absolute, 4),
            0x0E => instr!(ASL, Absolute, 6),
            0x10 => instr!(BPL, Relative, 2),
            0x11 => instr!(ORA, IndirectY, 5),
            0x15 => instr!(ORA, ZeroPageX, 4),
            0x16 => instr!(ASL, ZeroPageX, 6),
            0x18 => instr!(CLC, Implied, 2),
            0x19 => instr!(ORA, AbsoluteY, 4),
            0x1D => instr!(ORA, AbsoluteX, 4),
            0x1E => instr!(ASL, AbsoluteX, 6),
            0x20 => instr!(JSR, Absolute, 6),
            0x21 => instr!(AND, IndirectX, 6),
            0x24 => instr!(BIT, ZeroPage, 3),
            0x25 => instr!(AND, ZeroPage, 3),
            0x26 => instr!(ROL, ZeroPage, 5),
            0x28 => instr!(PLP, Implied, 4),
            0x29 => instr!(AND, Immediate, 2),
            0x2A => instr!(ROL, Implied, 2),
            0x2B => instr!(ANC, Immediate, 2),
            0x2C => instr!(BIT, Absolute, 4),
            0x2D => instr!(AND, Absolute, 4),
            0x2E => instr!(ROL, Absolute, 6),
            0x30 => instr!(BMI, Relative, 2),
            0x31 => instr!(AND, IndirectY, 5),
            0x35 => instr!(AND, ZeroPageX, 4),
            0x36 => instr!(ROL, ZeroPageX, 6),
            0x38 => instr!(SEC, Implied, 2),
            0x39 => instr!(AND, AbsoluteY, 4),
            0x3D => instr!(AND, AbsoluteX, 4),
            0x3E => instr!(ROL, AbsoluteX, 6),
            0x40 => instr!(RTI, Implied, 6),
            0x41 => instr!(EOR, IndirectX, 6),
            0x45 => instr!(EOR, ZeroPage, 3),
            0x46 => instr!(LSR, ZeroPage, 5),
            0x48 => instr!(PHA, Implied, 3),
            0x49 => instr!(EOR, Immediate, 2),
            0x4A => instr!(LSR, Implied, 2),
            0x4B => instr!(ALR, Immediate, 2),
            0x4C => instr!(JMP, Absolute, 3),
            0x4D => instr!(EOR, Absolute, 4),
            0x4E => instr!(LSR, Absolute, 6),
            0x50 => instr!(BVC, Relative, 2),
            0x51 => instr!(EOR, IndirectY, 5),
            0x55 => instr!(EOR, ZeroPageX, 4),
            0x56 => instr!(LSR, ZeroPageX, 6),
            0x58 => instr!(CLI, Implied, 2),
            0x59 => instr!(EOR, AbsoluteY, 4),
            0x5D => instr!(EOR, AbsoluteX, 4),
            0x5E => instr!(LSR, AbsoluteX, 6),
            0x60 => instr!(RTS, Implied, 6),
            0x61 => instr!(ADC, IndirectX, 6),
            0x65 => instr!(ADC, ZeroPage, 3),
            0x66 => instr!(ROR, ZeroPage, 5),
            0x68 => instr!(PLA, Implied, 4),
            0x69 => instr!(ADC, Immediate, 2),
            0x6A => instr!(ROR, Implied, 2),
            0x6B => instr!(ARR, Immediate, 2),
            0x6C => instr!(JMP, Indirect, 5),
            0x6D => instr!(ADC, Absolute, 4),
            0x6E => instr!(ROR, Absolute, 6),
            0x70 => instr!(BVS, Relative, 2),
            0x71 => instr!(ADC, IndirectY, 5),
            0x75 => instr!(ADC, ZeroPageX, 4),
            0x76 => instr!(ROR, ZeroPageX, 6),
            0x78 => instr!(SEI, Implied, 2),
            0x79 => instr!(ADC, AbsoluteY, 4),
            0x7D => instr!(ADC, AbsoluteX, 4),
            0x7E => instr!(ROR, AbsoluteX, 6),
            0x81 => instr!(STA, IndirectX, 6),
            0x84 => instr!(STY, ZeroPage, 3),
            0x85 => instr!(STA, ZeroPage, 3),
            0x86 => instr!(STX, ZeroPage, 3),
            0x88 => instr!(DEY, Implied, 2),
            0x8A => instr!(TXA, Implied, 2),
            0x8B => instr!(ANE, Immediate, 2),
            0x8C => instr!(STY, Absolute, 4),
            0x8D => instr!(STA, Absolute, 4),
            0x8E => instr!(STX, Absolute, 4),
            0x90 => instr!(BCC, Relative, 2),
            0x91 => instr!(STA, IndirectY, 5),
            0x94 => instr!(STY, ZeroPageX, 4),
            0x95 => instr!(STA, ZeroPageX, 4),
            0x96 => instr!(STX, ZeroPageY, 4),
            0x98 => instr!(TYA, Implied, 2),
            0x99 => instr!(STA, AbsoluteY, 4),
            0x9A => instr!(TXS, Implied, 2),
            0x9D => instr!(STA, AbsoluteX, 5),
            0xA0 => instr!(LDY, Immediate, 2),
            0xA1 => instr!(LDA, IndirectX, 6),
            0xA2 => instr!(LDX, Immediate, 2),
            0xA4 => instr!(LDY, ZeroPage, 3),
            0xA5 => instr!(LDA, ZeroPage, 3),
            0xA6 => instr!(LDX, ZeroPage, 3),
            0xA8 => instr!(TAY, Implied, 2),
            0xA9 => instr!(LDA, Immediate, 2),
            0xAA => instr!(TAX, Implied, 2),
            0xAC => instr!(LDY, Absolute, 4),
            0xAD => instr!(LDA, Absolute, 4),
            0xAE => instr!(LDX, Absolute, 4),
            0xB0 => instr!(BCS, Relative, 2),
            0xB1 => instr!(LDA, IndirectY, 5),
            0xB4 => instr!(LDY, ZeroPageX, 3),
            0xB5 => instr!(LDA, ZeroPageX, 3),
            0xB6 => instr!(LDX, ZeroPageY, 3),
            0xB8 => instr!(CLV, Implied, 2),
            0xB9 => instr!(LDA, AbsoluteY, 4),
            0xBA => instr!(TSX, Implied, 2),
            0xBC => instr!(LDY, AbsoluteX, 4),
            0xBD => instr!(LDA, AbsoluteX, 4),
            0xBE => instr!(LDX, AbsoluteY, 4),
            0xC0 => instr!(CPY, Immediate, 2),
            0xC1 => instr!(CMP, IndirectX, 6),
            0xC4 => instr!(CPY, ZeroPage, 3),
            0xC5 => instr!(CMP, ZeroPage, 3),
            0xC6 => instr!(DEC, ZeroPage, 5),
            0xC8 => instr!(INY, Implied, 2),
            0xC9 => instr!(CMP, Immediate, 2),
            0xCA => instr!(DEX, Implied, 2),
            0xCC => instr!(CPY, Absolute, 4),
            0xCD => instr!(CMP, Absolute, 4),
            0xCE => instr!(DEC, Absolute, 6),
            0xD0 => instr!(BNE, Relative, 2),
            0xD1 => instr!(CMP, IndirectY, 5),
            0xD5 => instr!(CMP, ZeroPageX, 4),
            0xD6 => instr!(DEC, ZeroPageX, 6),
            0xD8 => instr!(CLD, Implied, 2),
            0xD9 => instr!(CMP, AbsoluteY, 4),
            0xDD => instr!(CMP, AbsoluteX, 4),
            0xDE => instr!(DEC, AbsoluteX, 6),
            0xE0 => instr!(CPX, Immediate, 2),
            0xE1 => instr!(SBC, IndirectX, 6),
            0xE4 => instr!(CPX, ZeroPage, 3),
            0xE5 => instr!(SBC, ZeroPage, 3),
            0xE6 => instr!(INC, ZeroPage, 5),
            0xE8 => instr!(INX, Implied, 2),
            0xE9 => instr!(SBC, Immediate, 2),
            0xEA => instr!(NOP, Implied, 2),
            0xEC => instr!(CPX, Absolute, 4),
            0xED => instr!(SBC, Absolute, 4),
            0xEE => instr!(INC, Absolute, 6),
            0xF0 => instr!(BEQ, Relative, 2),
            0xF1 => instr!(SBC, IndirectY, 5),
            0xF5 => instr!(SBC, ZeroPageX, 4),
            0xF6 => instr!(INC, ZeroPageX, 6),
            0xF8 => instr!(SED, Implied, 2),
            0xF9 => instr!(SBC, AbsoluteY, 4),
            0xFD => instr!(SBC, AbsoluteX, 4),
            0xFE => instr!(INC, AbsoluteX, 6),
            opcode => Instruction {
                instr_type: InstrType::Invalid(opcode),
                addr_mode: AddrMode::Implied,
                cycles: 0,
            },
        }
    }
}

impl AddrMode {
    pub fn size(&self) -> u16 {
        match self {
            AddrMode::Implied => 1,
            AddrMode::Immediate(_) => 2,
            AddrMode::ZeroPage(_) => 2,
            AddrMode::ZeroPageX(_) => 2,
            AddrMode::ZeroPageY(_) => 2,
            AddrMode::Absolute(_) => 3,
            AddrMode::AbsoluteX(_) => 3,
            AddrMode::AbsoluteY(_) => 3,
            AddrMode::Indirect(_) => 3,
            AddrMode::IndirectX(_) => 2,
            AddrMode::IndirectY(_) => 2,
            AddrMode::Relative(_) => 2,
        }
    }

    pub fn resolve(&self, cpu: &NESCPU) -> (u8, u8) {
        match self {
            AddrMode::Implied => (0, 0),
            AddrMode::Immediate(val) => (*val, 0),
            AddrMode::ZeroPage(addr) => (NESBus::read(*addr as u16), 0),
            AddrMode::ZeroPageX(addr) => {
                let addr = addr.wrapping_add(cpu.reg_x);
                (NESBus::read(addr as u16), 0)
            }
            AddrMode::ZeroPageY(addr) => {
                let addr = addr.wrapping_add(cpu.reg_y);
                (NESBus::read(addr as u16), 0)
            }
            AddrMode::Absolute(addr) => (NESBus::read(*addr), 0),
            AddrMode::AbsoluteX(addr) => {
                let calc_addr = addr.wrapping_add(cpu.reg_x as u16);
                (
                    NESBus::read(calc_addr),
                    if addr & 0xFF00 != calc_addr & 0xFF00 {
                        1
                    } else {
                        0
                    },
                )
            }
            AddrMode::AbsoluteY(addr) => {
                let calc_addr = addr.wrapping_add(cpu.reg_y as u16);
                (
                    NESBus::read(calc_addr),
                    if addr & 0xFF00 != calc_addr & 0xFF00 {
                        1
                    } else {
                        0
                    },
                )
            }
            AddrMode::Indirect(addr) => {
                let lo = NESBus::read(*addr);
                let hi = NESBus::read(addr.wrapping_add(1));
                (u16::from_le_bytes([lo, hi]) as u8, 0)
            }
            AddrMode::IndirectX(addr) => {
                let ptr = addr.wrapping_add(cpu.reg_x);
                let lo = NESBus::read(ptr as u16);
                let hi = NESBus::read(ptr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]);
                (NESBus::read(addr), 0)
            }
            AddrMode::IndirectY(addr) => {
                let lo = NESBus::read(*addr as u16);
                let hi = NESBus::read(addr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]);
                let calc_addr = addr.wrapping_add(cpu.reg_y as u16);
                (
                    NESBus::read(calc_addr),
                    if addr & 0xFF00 != calc_addr & 0xFF00 {
                        1
                    } else {
                        0
                    },
                )
            }
            AddrMode::Relative(offset) => (
                cpu.reg_pc.wrapping_add(*offset as u16).wrapping_add(2) as u8,
                0,
            ),
        }
    }

    pub fn resolve_addr(&self, cpu: &NESCPU) -> Option<(u16, u8)> {
        match self {
            AddrMode::Implied => None,
            AddrMode::Immediate(_) => None,
            AddrMode::ZeroPage(_) => None,
            AddrMode::ZeroPageX(_) => None,
            AddrMode::ZeroPageY(_) => None,
            AddrMode::Absolute(addr) => Some((*addr, 0)),
            AddrMode::AbsoluteX(_) => None,
            AddrMode::AbsoluteY(_) => None,
            AddrMode::Indirect(addr) => {
                let lo = NESBus::read(*addr);
                let hi = NESBus::read(addr.wrapping_add(1));
                Some((u16::from_le_bytes([lo, hi]), 0))
            }
            AddrMode::IndirectX(_) => None,
            AddrMode::IndirectY(_) => None,
            AddrMode::Relative(offset) => {
                Some((cpu.reg_pc.wrapping_add(*offset as u16).wrapping_add(2), 0))
            }
        }
    }

    pub fn write(&self, cpu: &NESCPU, value: u8) {
        match self {
            AddrMode::Implied => {}
            AddrMode::Immediate(_) => {}
            AddrMode::ZeroPage(addr) => {
                NESBus::write(*addr as u16, value);
            }
            AddrMode::ZeroPageX(addr) => {
                let addr = addr.wrapping_add(cpu.reg_x);
                NESBus::write(addr as u16, value);
            }
            AddrMode::ZeroPageY(addr) => {
                let addr = addr.wrapping_add(cpu.reg_y);
                NESBus::write(addr as u16, value);
            }
            AddrMode::Absolute(addr) => {
                NESBus::write(*addr, value);
            }
            AddrMode::AbsoluteX(addr) => {
                let addr = addr.wrapping_add(cpu.reg_x as u16);
                NESBus::write(addr, value);
            }
            AddrMode::AbsoluteY(addr) => {
                let addr = addr.wrapping_add(cpu.reg_y as u16);
                NESBus::write(addr, value);
            }
            AddrMode::Indirect(_) => {}
            AddrMode::IndirectX(addr) => {
                let ptr = addr.wrapping_add(cpu.reg_x);
                let lo = NESBus::read(ptr as u16);
                let hi = NESBus::read(ptr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]);
                NESBus::write(addr, value);
            }
            AddrMode::IndirectY(addr) => {
                let lo = NESBus::read(*addr as u16);
                let hi = NESBus::read(addr.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]).wrapping_add(cpu.reg_y as u16);
                NESBus::write(addr, value);
            }
            AddrMode::Relative(_) => {}
        }
    }
}
