use core::ptr::{read_volatile, write_volatile};

use crate::{critical, drivers::pci::PCIDevice, info};

pub const VIRT_QUEUE_SIZE: usize = 64;

#[repr(C)]
pub struct VirtQDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
pub struct VirtQAvail {
    flags: u16,
    idx: u16,
    ring: [u16; VIRT_QUEUE_SIZE],
    used_event: u16,
}

#[repr(C)]
pub struct VirtQUsed {
    flags: u16,
    idx: u16,
    ring: [VirtQUsedElem; VIRT_QUEUE_SIZE],
    avail_event: u16,
}

#[repr(C)]
pub struct VirtQUsedElem {
    id: u32,
    len: u32,
}

impl VirtQDesc {
    pub const VIRTQ_DESC_F_NEXT: u16 = 1;
    pub const VIRTQ_DESC_F_WRITE: u16 = 2;
    pub const VIRTQ_DESC_F_INDIRECT: u16 = 4;
}

const VIRTQ_ALIGN: usize = 4096;
const VIRTQ_PADDING: usize = VIRTQ_ALIGN
    - (size_of::<VirtQDesc>() * VIRT_QUEUE_SIZE + size_of::<VirtQAvail>()) % VIRTQ_ALIGN;

#[repr(C, align(4096))]
pub struct VirtQ {
    desc: [VirtQDesc; VIRT_QUEUE_SIZE],
    avail: VirtQAvail,
    padding: [u8; VIRTQ_PADDING],
    used: VirtQUsed,
}

pub struct VirtIODevice {
    pci_device: PCIDevice,
    pub common_config: &'static mut PCICommonConfig,
    notify_base: *mut u8,
    notify_off_multiplier: u32,
}

#[repr(C)]
pub struct PCICommonConfig {
    /* About the whole device. */
    pub device_feature_select: u32,
    pub device_feature: u32,
    pub driver_feature_select: u32,
    pub driver_feature: u32,
    pub config_msix_vector: u16,
    pub num_queues: u16,
    pub device_status: u8,
    pub config_generation: u8,

    /* About a specific virtqueue. */
    pub queue_select: u16,
    pub queue_size: u16,
    pub queue_msix_vector: u16,
    pub queue_enable: u16,
    pub queue_notify_off: u16,
    pub queue_desc: u64,
    pub queue_driver: u64,
    pub queue_device: u64,
    pub queue_notif_config_data: u16,
    pub queue_reset: u16,

    /* About the administration virtqueue. */
    pub admin_queue_index: u16,
    pub admin_queue_num: u16,
}

impl VirtIODevice {
    const VIRTIO_VENDOR_ID: u16 = 0x1AF4;

    pub fn new(device_id: u16) -> Option<Self> {
        let pci_device = PCIDevice::find_device(Self::VIRTIO_VENDOR_ID, device_id)?;
        let (common_config_ptr, notify_base, notify_off_multiplier) =
            Self::common_config(&pci_device)?;
        let common_config = unsafe { &mut *common_config_ptr };
        Some(Self {
            pci_device: pci_device,
            common_config,
            notify_base,
            notify_off_multiplier,
        })
    }

    const VENDOR_SPECIFIC_CAP_ID: u8 = 0x09;

    const PCI_CAP_COMMON_CFG: u8 = 1;
    const PCI_CAP_NOTIFY_CFG: u8 = 2;

    fn common_config(pci_device: &PCIDevice) -> Option<(*mut PCICommonConfig, *mut u8, u32)> {
        let mut pointer = pci_device.capabilities_pointer()?;
        let mut config = None;
        let mut notify_base = None;

        while pointer != 0 {
            let cap_id = pci_device.read_config::<u8>(pointer);
            let cap_next = pci_device.read_config::<u8>(pointer + 1);
            let cap_len = pci_device.read_config::<u8>(pointer + 2);
            info!(
                DRV,
                "Found a capability: cap_id={:#04X} next_pointer={:#04X} len={:#04X}",
                cap_id,
                cap_next,
                cap_len
            );
            if cap_id == Self::VENDOR_SPECIFIC_CAP_ID {
                let bar_index = pci_device.read_config::<u8>(pointer + 4);
                let offset = pci_device.read_config::<u32>(pointer + 8);
                let length = pci_device.read_config::<u32>(pointer + 12);

                let cfg_type = pci_device.read_config::<u8>(pointer + 3);
                info!(
                    DRV,
                    "Found a vendor specific capability: type={:#04X}", cfg_type
                );
                match cfg_type {
                    Self::PCI_CAP_COMMON_CFG => {
                        info!(
                            DRV,
                            "VIRTIO_PCI_CAP_COMMON_CFG: bar_index={} offset={:#010X} length={:#010X}",
                            bar_index,
                            offset,
                            length
                        );

                        let address_lo =
                            pci_device.read_config::<u32>(0x10 + (bar_index as u8 * 4)) as u64;
                        let address_hi = pci_device
                            .read_config::<u32>(0x10 + ((bar_index + 1) as u8 * 4))
                            as u64;

                        // Through monitoring QEMU, it seems that the base address is always 16-byte aligned.
                        // I'm not sure it is true.
                        let address = ((address_hi << 32) | (address_lo & !0xF)) + offset as u64;
                        info!(DRV, "PCI common config address: {:#018X}", address);
                        config = Some(address as *mut PCICommonConfig);
                    }
                    Self::PCI_CAP_NOTIFY_CFG => {
                        let notify_off_multiplier = pci_device.read_config::<u32>(pointer + 16);

                        info!(
                            DRV,
                            "VIRTIO_PCI_CAP_NOTIFY_CFG: bar_index={} offset={:#010X} length={:#010X} multiplier={}",
                            bar_index,
                            offset,
                            length,
                            notify_off_multiplier
                        );

                        let address_lo =
                            pci_device.read_config::<u32>(0x10 + (bar_index as u8 * 4)) as u64;
                        let address_hi = pci_device
                            .read_config::<u32>(0x10 + ((bar_index + 1) as u8 * 4))
                            as u64;

                        // Through monitoring QEMU, it seems that the base address is always 16-byte aligned.
                        // I'm not sure it is true.
                        let address = ((address_hi << 32) | (address_lo & !0xF)) + offset as u64;
                        info!(DRV, "PCI notify base address: {:#018X}", address);
                        notify_base = Some((address as *mut u8, notify_off_multiplier));
                    }
                    _ => {}
                }
            }
            pointer = cap_next;
        }
        match (config, notify_base) {
            (Some(cfg), Some((notify_base, notify_off_multiplier))) => {
                Some((cfg, notify_base, notify_off_multiplier))
            }
            _ => None,
        }
    }

    const ACKNOWLEDGE: u8 = 1;
    const DRIVER: u8 = 2;
    const DRIVER_OK: u8 = 4;
    const FEATURES_OK: u8 = 8;
    const DEVICE_NEEDS_RESET: u8 = 64;
    const FAILED: u8 = 128;

    pub fn init(&mut self) {
        info!(DRV, "Num queues: {}", self.common_config.num_queues);

        // Reset the device.
        unsafe {
            write_volatile(&mut self.common_config.device_status, 0);
        }

        // Acknowledge the device.
        unsafe {
            write_volatile(
                &mut self.common_config.device_status,
                Self::FEATURES_OK | Self::ACKNOWLEDGE,
            );
        }

        let status = unsafe { read_volatile(&self.common_config.device_status) };
        if (status & Self::ACKNOWLEDGE) == 0 {
            critical!(DRV, "Failed to acknowledge the device.");
        }
        if (status & Self::FEATURES_OK) == 0 {
            critical!(DRV, "The device does not support FEATURES_OK.");
        }

        // This is a driver.
        unsafe {
            write_volatile(
                &mut self.common_config.device_status,
                self.common_config.device_status | Self::DRIVER,
            );
        }
    }

    pub fn init_queue(&mut self, queue_idx: u16, queue: &mut VirtQ) -> *mut u8 {
        // Select the queue.
        unsafe {
            write_volatile(&mut self.common_config.queue_select, queue_idx);
        }

        // Set the queue size.
        unsafe {
            write_volatile(&mut self.common_config.queue_size, VIRT_QUEUE_SIZE as u16);
        }

        unsafe {
            write_volatile(
                &mut self.common_config.queue_desc,
                queue.desc.as_ptr() as u64,
            );
        }
        unsafe {
            write_volatile(
                &mut self.common_config.queue_driver,
                &mut queue.avail as *mut _ as u64,
            );
        }
        unsafe {
            write_volatile(
                &mut self.common_config.queue_device,
                &mut queue.used as *mut _ as u64,
            );
        }

        // Enable the queue.
        unsafe {
            write_volatile(&mut self.common_config.queue_enable, 1);
        }

        // Get notify address.
        let offset = unsafe { read_volatile(&mut self.common_config.queue_notify_off) };
        let notify_address = unsafe {
            self.notify_base
                .add((offset as u32 * self.notify_off_multiplier) as usize)
        };

        info!(
            DRV,
            "Initialized queue {}: notify_address={:p}", queue_idx, notify_address
        );

        notify_address
    }

    pub fn driver_ok(&mut self) {
        unsafe {
            write_volatile(
                &mut self.common_config.device_status,
                self.common_config.device_status | Self::DRIVER_OK,
            );
        }
    }
}

pub mod block;
pub mod sound;
