use spin::{lazy::Lazy, mutex::Mutex};
use uart_16550::SerialPort;

static SERIAL: Lazy<Mutex<Serial>> = Lazy::new(|| {
    let mut serial_port = unsafe { SerialPort::new(Serial::SERIAL_PORT_IO_ADDR) };
    serial_port.init();
    Mutex::new(Serial { serial_port })
});

pub struct Serial {
    serial_port: SerialPort,
}

impl Serial {
    const SERIAL_PORT_IO_ADDR: u16 = 0x3F8;

    const SPECIAL_CTRL_CHAR: u8 = 0x93;

    pub fn communicate<F>(handler: F)
    where
        F: FnOnce(&mut Self),
    {
        let mut port = SERIAL.lock();
        port.serial_port.send_raw(Self::SPECIAL_CTRL_CHAR);
        handler(&mut port);
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

    pub fn write_u16(&mut self, data: u16) {
        self.write(&data.to_le_bytes());
    }

    pub fn write_u32(&mut self, data: u32) {
        self.write(&data.to_le_bytes());
    }

    pub fn read_u8(&mut self) -> u8 {
        self.serial_port.receive()
    }

    pub fn read_u16(&mut self) -> u16 {
        let mut buf = [0; 2];
        self.read(&mut buf);
        u16::from_le_bytes(buf)
    }

    pub fn read_u32(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.read(&mut buf);
        u32::from_le_bytes(buf)
    }
}
