use alloc::vec;
use crc::{Crc, CRC_32_ISCSI};
use postcard::{from_bytes_crc32, to_allocvec_crc32};
use spin::{lazy::Lazy, mutex::Mutex};
use uart_16550::SerialPort;

use crate::{
    error, info,
    nes::{cartridge::Cartridge, cpu::NESCPU, ppu::NESPPU},
};

static SERIAL: Lazy<Mutex<Serial>> = Lazy::new(|| {
    let mut serial_port = unsafe { SerialPort::new(Serial::SERIAL_PORT_IO_ADDR) };
    serial_port.init();
    Mutex::new(Serial { serial_port })
});

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SerialRequest {
    #[allow(dead_code)]
    Active = 1,
    SaveState = 2,
    LoadState = 3,
}

impl SerialRequest {
    pub fn send(&self, serial: &mut Serial) {
        serial.write_u8(*self as u8);
    }
}

pub struct Serial {
    serial_port: SerialPort,
}

impl Serial {
    const SERIAL_PORT_IO_ADDR: u16 = 0x3F8;

    const SPECIAL_CTRL_CHAR: u8 = 0x93;

    pub fn communicate<F, T>(handler: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let mut port = SERIAL.lock();
        port.serial_port.send_raw(Self::SPECIAL_CTRL_CHAR);
        handler(&mut port)
    }

    pub fn save_state(
        &mut self,
        cpu: &NESCPU,
        ppu: &NESPPU,
        cartridge: &Cartridge,
    ) -> Result<(), ()> {
        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        SerialRequest::SaveState.send(self);

        info!(COM, "Request to save state");

        let serialized_cpu = to_allocvec_crc32(cpu, crc.digest());
        let serialized_cpu = serialized_cpu.map_err(|_| {
            error!(COM, "Failed to serialize CPU state");
            ()
        })?;

        self.write_u32(serialized_cpu.len() as u32);
        self.write(&serialized_cpu);

        info!(COM, "Sent CPU state ({} bytes)", serialized_cpu.len());

        let serialized_ppu = to_allocvec_crc32(ppu, crc.digest());
        let serialized_ppu = serialized_ppu.map_err(|_| {
            error!(COM, "Failed to serialize PPU state");
            ()
        })?;

        self.write_u32(serialized_ppu.len() as u32);
        self.write(&serialized_ppu);

        info!(COM, "Sent PPU state ({} bytes)", serialized_ppu.len());

        let serialized_cartridge = to_allocvec_crc32(cartridge, crc.digest());
        let serialized_cartridge = serialized_cartridge.map_err(|_| {
            error!(COM, "Failed to serialize cartridge state");
            ()
        })?;

        self.write_u32(serialized_cartridge.len() as u32);
        self.write(&serialized_cartridge);
        info!(
            COM,
            "Sent cartridge state ({} bytes)",
            serialized_cartridge.len()
        );

        Ok(())
    }

    pub fn load_state(
        &mut self,
        cpu: &mut NESCPU,
        ppu: &mut NESPPU,
        cartridge: &mut Cartridge,
    ) -> Result<(), ()> {
        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        SerialRequest::LoadState.send(self);

        info!(COM, "Request to load saved state");

        let cpu_size = self.read_u32() as usize;
        let mut cpu_buf = vec![0; cpu_size];
        self.read(&mut cpu_buf);

        info!(COM, "Received CPU state ({} bytes)", cpu_size);

        *cpu = from_bytes_crc32(&cpu_buf, crc.digest()).map_err(|_| {
            error!(COM, "Failed to deserialize CPU state");
            ()
        })?;

        let ppu_size = self.read_u32() as usize;
        let mut ppu_buf = vec![0; ppu_size];
        self.read(&mut ppu_buf);

        info!(COM, "Received PPU state ({} bytes)", ppu_size);

        *ppu = from_bytes_crc32(&ppu_buf, crc.digest()).map_err(|_| {
            error!(COM, "Failed to deserialize PPU state");
            ()
        })?;

        let cartridge_size = self.read_u32() as usize;
        let mut cartridge_buf = vec![0; cartridge_size];
        self.read(&mut cartridge_buf);

        info!(SYS, "Received cartridge state ({} bytes)", cartridge_size);

        *cartridge = from_bytes_crc32(&cartridge_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize cartridge state");
            ()
        })?;

        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) {
        for &b in data {
            self.serial_port.send_raw(b);
        }
    }

    pub fn read(&mut self, data: &mut [u8]) {
        for b in data {
            *b = self.serial_port.receive();
        }
    }

    pub fn write_u8(&mut self, data: u8) {
        self.serial_port.send_raw(data);
    }

    pub fn write_u32(&mut self, data: u32) {
        self.write(&data.to_le_bytes());
    }

    pub fn read_u32(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.read(&mut buf);
        u32::from_le_bytes(buf)
    }
}
