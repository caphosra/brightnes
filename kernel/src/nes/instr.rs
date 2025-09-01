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
    // Illegal
    Illegal(u8),
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
        match NESBus::read(pc) {
            0x00 => Instruction {
                instr_type: InstrType::BRK,
                addr_mode: AddrMode::Implied,
                cycles: 7,
            },
            0x01 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x05 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x06 => Instruction {
                instr_type: InstrType::ASL,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x08 => Instruction {
                instr_type: InstrType::PHP,
                addr_mode: AddrMode::Implied,
                cycles: 3,
            },
            0x09 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0x0A => Instruction {
                instr_type: InstrType::ASL,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x0D => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x0E => Instruction {
                instr_type: InstrType::ASL,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x10 => Instruction {
                instr_type: InstrType::BPL,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0x11 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x15 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x16 => Instruction {
                instr_type: InstrType::ASL,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x18 => Instruction {
                instr_type: InstrType::CLC,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x19 => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x1D => Instruction {
                instr_type: InstrType::ORA,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x1E => Instruction {
                instr_type: InstrType::ASL,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x20 => Instruction {
                instr_type: InstrType::JSR,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x21 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x24 => Instruction {
                instr_type: InstrType::BIT,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x25 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x26 => Instruction {
                instr_type: InstrType::ROL,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x28 => Instruction {
                instr_type: InstrType::PLP,
                addr_mode: AddrMode::Implied,
                cycles: 4,
            },
            0x29 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0x2A => Instruction {
                instr_type: InstrType::ROL,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x2C => Instruction {
                instr_type: InstrType::BIT,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x2D => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x2E => Instruction {
                instr_type: InstrType::ROL,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x30 => Instruction {
                instr_type: InstrType::BMI,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0x31 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x35 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x36 => Instruction {
                instr_type: InstrType::ROL,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x38 => Instruction {
                instr_type: InstrType::SEC,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x39 => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x3D => Instruction {
                instr_type: InstrType::AND,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x3E => Instruction {
                instr_type: InstrType::ROL,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x40 => Instruction {
                instr_type: InstrType::RTI,
                addr_mode: AddrMode::Implied,
                cycles: 6,
            },
            0x41 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x45 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x46 => Instruction {
                instr_type: InstrType::LSR,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x48 => Instruction {
                instr_type: InstrType::PHA,
                addr_mode: AddrMode::Implied,
                cycles: 3,
            },
            0x49 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0x4A => Instruction {
                instr_type: InstrType::LSR,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x4C => Instruction {
                instr_type: InstrType::JMP,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 3,
            },
            0x4D => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x4E => Instruction {
                instr_type: InstrType::LSR,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x50 => Instruction {
                instr_type: InstrType::BVC,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0x51 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x55 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x56 => Instruction {
                instr_type: InstrType::LSR,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x58 => Instruction {
                instr_type: InstrType::CLI,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x59 => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x5D => Instruction {
                instr_type: InstrType::EOR,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x5E => Instruction {
                instr_type: InstrType::LSR,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x60 => Instruction {
                instr_type: InstrType::RTS,
                addr_mode: AddrMode::Implied,
                cycles: 6,
            },
            0x61 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x65 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x66 => Instruction {
                instr_type: InstrType::ROR,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x68 => Instruction {
                instr_type: InstrType::PLA,
                addr_mode: AddrMode::Implied,
                cycles: 4,
            },
            0x69 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0x6A => Instruction {
                instr_type: InstrType::ROR,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x6C => Instruction {
                instr_type: InstrType::JMP,
                addr_mode: AddrMode::Indirect(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 5,
            },
            0x6D => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x6E => Instruction {
                instr_type: InstrType::ROR,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x70 => Instruction {
                instr_type: InstrType::BVS,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0x71 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x75 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x76 => Instruction {
                instr_type: InstrType::ROR,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x78 => Instruction {
                instr_type: InstrType::SEI,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x79 => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x7D => Instruction {
                instr_type: InstrType::ADC,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x7E => Instruction {
                instr_type: InstrType::ROR,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0x81 => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0x84 => Instruction {
                instr_type: InstrType::STY,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x85 => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x86 => Instruction {
                instr_type: InstrType::STX,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0x88 => Instruction {
                instr_type: InstrType::DEY,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x8A => Instruction {
                instr_type: InstrType::TXA,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x8C => Instruction {
                instr_type: InstrType::STY,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x8D => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x8E => Instruction {
                instr_type: InstrType::STX,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x90 => Instruction {
                instr_type: InstrType::BCC,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0x91 => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0x94 => Instruction {
                instr_type: InstrType::STY,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x95 => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x96 => Instruction {
                instr_type: InstrType::STX,
                addr_mode: AddrMode::ZeroPageY(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0x98 => Instruction {
                instr_type: InstrType::TYA,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x99 => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0x9A => Instruction {
                instr_type: InstrType::TXS,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0x9D => Instruction {
                instr_type: InstrType::STA,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 5,
            },
            0xA0 => Instruction {
                instr_type: InstrType::LDY,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xA1 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0xA2 => Instruction {
                instr_type: InstrType::LDX,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xA4 => Instruction {
                instr_type: InstrType::LDY,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xA5 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xA6 => Instruction {
                instr_type: InstrType::LDX,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xA8 => Instruction {
                instr_type: InstrType::TAY,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xA9 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xAA => Instruction {
                instr_type: InstrType::TAX,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xAC => Instruction {
                instr_type: InstrType::LDY,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xAD => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xAE => Instruction {
                instr_type: InstrType::LDX,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xB0 => Instruction {
                instr_type: InstrType::BCS,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0xB1 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0xB4 => Instruction {
                instr_type: InstrType::LDY,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xB5 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xB6 => Instruction {
                instr_type: InstrType::LDX,
                addr_mode: AddrMode::ZeroPageY(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xB8 => Instruction {
                instr_type: InstrType::CLV,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xB9 => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xBA => Instruction {
                instr_type: InstrType::TSX,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xBC => Instruction {
                instr_type: InstrType::LDY,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xBD => Instruction {
                instr_type: InstrType::LDA,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xBE => Instruction {
                instr_type: InstrType::LDX,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xC0 => Instruction {
                instr_type: InstrType::CPY,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xC1 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0xC4 => Instruction {
                instr_type: InstrType::CPY,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xC5 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xC6 => Instruction {
                instr_type: InstrType::DEC,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0xC8 => Instruction {
                instr_type: InstrType::INY,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xC9 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xCA => Instruction {
                instr_type: InstrType::DEX,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xCC => Instruction {
                instr_type: InstrType::CPY,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xCD => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xCE => Instruction {
                instr_type: InstrType::DEC,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0xD0 => Instruction {
                instr_type: InstrType::BNE,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0xD1 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0xD5 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0xD6 => Instruction {
                instr_type: InstrType::DEC,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0xD8 => Instruction {
                instr_type: InstrType::CLD,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xD9 => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xDD => Instruction {
                instr_type: InstrType::CMP,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xDE => Instruction {
                instr_type: InstrType::DEC,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0xE0 => Instruction {
                instr_type: InstrType::CPX,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xE1 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::IndirectX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0xE4 => Instruction {
                instr_type: InstrType::CPX,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xE5 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 3,
            },
            0xE6 => Instruction {
                instr_type: InstrType::INC,
                addr_mode: AddrMode::ZeroPage(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0xE8 => Instruction {
                instr_type: InstrType::INX,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xE9 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::Immediate(NESBus::read(pc + 1)),
                cycles: 2,
            },
            0xEA => Instruction {
                instr_type: InstrType::NOP,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xEC => Instruction {
                instr_type: InstrType::CPX,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xED => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xEE => Instruction {
                instr_type: InstrType::INC,
                addr_mode: AddrMode::Absolute(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            0xF0 => Instruction {
                instr_type: InstrType::BEQ,
                addr_mode: AddrMode::Relative(NESBus::read(pc + 1) as i8),
                cycles: 2,
            },
            0xF1 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::IndirectY(NESBus::read(pc + 1)),
                cycles: 5,
            },
            0xF5 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 4,
            },
            0xF6 => Instruction {
                instr_type: InstrType::INC,
                addr_mode: AddrMode::ZeroPageX(NESBus::read(pc + 1)),
                cycles: 6,
            },
            0xF8 => Instruction {
                instr_type: InstrType::SED,
                addr_mode: AddrMode::Implied,
                cycles: 2,
            },
            0xF9 => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::AbsoluteY(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xFD => Instruction {
                instr_type: InstrType::SBC,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 4,
            },
            0xFE => Instruction {
                instr_type: InstrType::INC,
                addr_mode: AddrMode::AbsoluteX(u16::from_le_bytes([
                    NESBus::read(pc + 1),
                    NESBus::read(pc + 2),
                ])),
                cycles: 6,
            },
            opcode => Instruction {
                instr_type: InstrType::Illegal(opcode),
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
