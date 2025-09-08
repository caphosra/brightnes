use crate::{
    logger::NESResult,
    nes::{
        cartridge::Cartridge,
        pad::PADS,
        ppu::{NES_PPU, OAM_DMA_ADDR},
        ram::NES_RAM,
    },
    warn,
};

pub struct CPUBus;

impl CPUBus {
    pub fn read(addr: u16, cartridge: &mut Cartridge) -> NESResult<u8> {
        if addr < 0x2000 {
            // RAM
            let ram = NES_RAM.read();
            Ok(ram.ram[addr as usize & 0x7FF])
        } else if addr < 0x4000 {
            // PPU
            let mut ppu = NES_PPU.write();
            ppu.read_reg(addr, cartridge)
        } else if addr == OAM_DMA_ADDR {
            // OAM DMA
            let mut ppu = NES_PPU.write();
            ppu.read_reg(addr, cartridge)
        } else if addr < 0x4016 {
            // APU
            warn!(APU, "Attempt to read from APU: {:#06X}", addr);
            Ok(0)
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].read().map(|v| v as u8)
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].read().map(|v| v as u8)
        } else {
            // Cartridge
            cartridge.read_cpu_mem(addr)
        }
    }

    pub fn write(addr: u16, data: u8, cartridge: &mut Cartridge) -> NESResult<()> {
        if addr < 0x2000 {
            // RAM
            let mut ram = NES_RAM.write();
            ram.ram[addr as usize & 0x7FF] = data;
            Ok(())
        } else if addr < 0x4000 {
            // PPU
            let mut ppu = NES_PPU.write();
            ppu.write_reg(addr, data, cartridge)
        } else if addr == OAM_DMA_ADDR {
            // OAM DMA
            let mut ppu = NES_PPU.write();
            ppu.write_reg(addr, data, cartridge)
        } else if addr < 0x4016 {
            // APU
            warn!(APU, "Attempt to write to APU: {:#06X}", addr);
            Ok(())
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].write(data & 1 == 1)
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].write(data & 1 == 1)
        } else {
            // Cartridge
            cartridge.write_cpu_mem(addr, data)
        }
    }
}
