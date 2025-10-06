use brightnes_common::serial::SerialRequest;
use spin::{lazy::Lazy, mutex::Mutex};
use uart_16550::SerialPort;

use crate::error;

static SERIAL: Lazy<Mutex<Serial>> = Lazy::new(|| {
    let mut serial_port = unsafe { SerialPort::new(Serial::SERIAL_PORT_IO_ADDR) };
    serial_port.init();
    Mutex::new(Serial { serial_port })
});

trait SerialSend {
    fn send(&self, serial: &mut Serial);
}

impl SerialSend for SerialRequest {
    fn send(&self, serial: &mut Serial) {
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

    pub fn save_ram(&mut self, data: &[u8]) -> Result<(), ()> {
        SerialRequest::SaveRAM.send(self);

        self.write_u32(data.len() as u32);
        self.write(data);

        Ok(())
    }

    pub fn load_ram(&mut self, buffer: &mut [u8]) -> Result<(), ()> {
        SerialRequest::LoadRAM.send(self);

        let size = self.read_u32() as usize;
        if size != buffer.len() {
            error!(
                COM,
                "RAM size mismatch: expected {}, got {}",
                buffer.len(),
                size
            );

            Err(())
        } else {
            self.read(buffer);

            Ok(())
        }
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
