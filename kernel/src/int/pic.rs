use x86_64::instructions::port::Port;

pub const PIC1_CMD: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_CMD: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;

pub const PIC_1_OFFSET: u8 = 0x20;
pub const PIC_2_OFFSET: u8 = 0x28;

pub struct PIC;

impl PIC {
    pub fn remap_irq1() {
        let mut pic1_cmd = Port::<u8>::new(PIC1_CMD);
        let mut pic1_data = Port::<u8>::new(PIC1_DATA);
        let mut pic2_cmd = Port::<u8>::new(PIC2_CMD);
        let mut pic2_data = Port::<u8>::new(PIC2_DATA);

        // Save the current mask.
        let a1 = unsafe { pic1_data.read() };
        let a2 = unsafe { pic2_data.read() };

        // Initialize the port.
        unsafe {
            pic1_cmd.write(0x11);
            pic2_cmd.write(0x11);
        }

        // Set the offsets.
        unsafe {
            pic1_data.write(PIC_1_OFFSET);
            pic2_data.write(PIC_2_OFFSET);
        }

        // Cascading settings
        unsafe {
            pic1_data.write(0x04);
            pic2_data.write(0x02);
        }

        // ICW4
        unsafe {
            pic1_data.write(0x01);
            pic2_data.write(0x01);
        }

        // Reconfigure the mask.
        unsafe {
            pic1_data.write(a1 & !(1 << 1));
            pic2_data.write(a2);
        }
    }

    pub fn eoi(irq: u8) {
        let port = if irq >= 8 { PIC2_CMD } else { PIC1_CMD };
        unsafe {
            Port::<u8>::new(port).write(0x20);
        }
    }
}
