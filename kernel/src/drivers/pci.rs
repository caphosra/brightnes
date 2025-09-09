use alloc::vec::Vec;
use x86_64::{instructions::port::Port, structures::port::PortRead};

pub struct PCIDevice {
    pub bus_number: u8,
    pub device_number: u8,
    pub function_number: u8,
}

const VIRTIO_VENDOR_ID: u16 = 0x1AF4;

impl PCIDevice {
    const CONFIG_ADDR: u16 = 0xCF8;
    const CONFIG_DATA: u16 = 0xCFC;

    const STATUS_REG_OFFSET: u8 = 0x06;
    const CAP_PTR_REG_OFFSET: u8 = 0x34;

    pub fn find_device(vendor_id: u16, device_id: u16) -> Vec<PCIDevice> {
        let mut devices = Vec::new();
        for bus in 0..=0xFF {
            for device in 0..0x20 {
                for function in 0..0x8 {
                    // Write to CONFIG_ADDR port
                    let mut config_addr = Port::<u32>::new(Self::CONFIG_ADDR);
                    unsafe {
                        config_addr.write(
                            (1u32 << 31)
                                | ((bus as u32) << 16)
                                | ((device as u32) << 11)
                                | ((function as u32) << 8),
                        )
                    };

                    let mut config_data = Port::<u32>::new(Self::CONFIG_DATA);
                    let data = unsafe { config_data.read() };
                    let found_vendor_id = (data & 0xFFFF) as u16;
                    let found_device_id = (data >> 16) as u16;
                    if found_vendor_id == vendor_id && found_device_id == device_id {
                        devices.push(PCIDevice {
                            bus_number: bus,
                            device_number: device,
                            function_number: function,
                        });
                    }
                }
            }
        }
        devices
    }

    pub fn read_config<T>(&self, offset: u8) -> T
    where
        T: PortRead,
    {
        let mut config_addr = Port::<u32>::new(Self::CONFIG_ADDR);
        unsafe {
            config_addr.write(
                (1u32 << 31)
                    | ((self.bus_number as u32) << 16)
                    | ((self.device_number as u32) << 11)
                    | ((self.function_number as u32) << 8)
                    | ((offset as u32) & 0xFC),
            )
        };

        let mut config_data = Port::<T>::new(Self::CONFIG_DATA);
        unsafe { config_data.read() }
    }

    pub fn capabilities_pointer(&self) -> Option<u8> {
        let status = self.read_config::<u16>(Self::STATUS_REG_OFFSET);
        if (status & (1 << 4)) == 0 {
            // The device does not support capabilities list.
            None
        } else {
            Some(self.read_config::<u8>(Self::CAP_PTR_REG_OFFSET))
        }
    }
}
