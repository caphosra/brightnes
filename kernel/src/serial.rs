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

    pub fn communicate<F, T>(handler: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let mut port = SERIAL.lock();
        handler(&mut port)
    }

    pub fn write(&mut self, data: &[u8]) {
        for &b in data {
            self.serial_port.send_raw(b);
        }
    }

    pub fn _read(&mut self, data: &mut [u8]) {
        for b in data {
            *b = self.serial_port.receive();
        }
    }
}
