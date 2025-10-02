use crate::nes::{
    apu::APU,
    cartridge::Cartridge,
    cpu::CPU,
    pad::PADS,
    ppu::{PPU, OAM_DMA_ADDR},
};

pub struct CPUBus;

impl CPUBus {
    pub fn read(
        addr: u16,
        cpu: &CPU,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> u8 {
        if addr < 0x2000 {
            // RAM
            cpu.ram.read(addr & 0x7FF)
        } else if addr < 0x4000 {
            // PPU
            ppu.read_reg(addr, cartridge)
        } else if addr == OAM_DMA_ADDR {
            // OAM DMA
            ppu.read_reg(addr, cartridge)
        } else if addr < 0x4016 {
            // APU
            apu.read_reg(addr)
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].read() as u8
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].read() as u8
        } else {
            // Cartridge
            cartridge.read_cpu_mem(addr)
        }
    }

    pub fn write(
        addr: u16,
        data: u8,
        cpu: &mut CPU,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) {
        if addr < 0x2000 {
            // RAM
            cpu.ram.write(addr & 0x7FF, data);
        } else if addr < 0x4000 {
            // PPU
            ppu.write_reg(addr, data, cpu, cartridge);
        } else if addr == OAM_DMA_ADDR {
            // OAM DMA
            ppu.write_reg(addr, data, cpu, cartridge);
        } else if addr < 0x4016 {
            // APU
            apu.write_reg(addr, data);
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].write(data & 1 == 1);
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].write(data & 1 == 1);
        } else {
            // Cartridge
            cartridge.write_cpu_mem(addr, data);
        }
    }
}
