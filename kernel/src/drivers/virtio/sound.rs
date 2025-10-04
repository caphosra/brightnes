use crate::{
    drivers::virtio::{VirtIODevice, VirtQ},
    mem::MemoryAllocator,
};

#[repr(C)]
struct SoundConfig {
    jacks: u32,
    streams: u32,
    chmaps: u32,
    controls: u32,
}

pub struct VirtSoundDevice<'a> {
    base_driver: VirtIODevice,

    control_queue: &'a mut VirtQ,
    control_queue_notify_addr: *mut u8,

    event_queue: &'a mut VirtQ,
    event_queue_notify_addr: *mut u8,

    tx_queue: &'a mut VirtQ,
    tx_queue_notify_addr: *mut u8,

    rx_queue: &'a mut VirtQ,
    rx_queue_notify_addr: *mut u8,

    config: &'a mut SoundConfig,
}

impl<'a> VirtSoundDevice<'a> {
    pub const VIRTIO_SOUND_DEVICE_ID: u16 = 0x1059;

    pub fn new() -> Option<Self> {
        let mut base_driver = VirtIODevice::new(Self::VIRTIO_SOUND_DEVICE_ID)?;

        let config = unsafe { (base_driver.device_specific as *mut SoundConfig).as_mut() }?;

        let control_queue = MemoryAllocator::alloc::<VirtQ>();
        let control_queue = unsafe { control_queue.as_mut() }?;

        let event_queue = MemoryAllocator::alloc::<VirtQ>();
        let event_queue = unsafe { event_queue.as_mut() }?;

        let tx_queue = MemoryAllocator::alloc::<VirtQ>();
        let tx_queue = unsafe { tx_queue.as_mut() }?;

        let rx_queue = MemoryAllocator::alloc::<VirtQ>();
        let rx_queue = unsafe { rx_queue.as_mut() }?;

        base_driver.init();
        let control_queue_notify_addr = base_driver.init_queue(0, control_queue);
        let event_queue_notify_addr = base_driver.init_queue(1, event_queue);
        let tx_queue_notify_addr = base_driver.init_queue(2, tx_queue);
        let rx_queue_notify_addr = base_driver.init_queue(3, rx_queue);
        base_driver.driver_ok();

        Some(Self {
            base_driver,
            control_queue,
            control_queue_notify_addr,
            event_queue,
            event_queue_notify_addr,
            tx_queue,
            tx_queue_notify_addr,
            rx_queue,
            rx_queue_notify_addr,
            config,
        })
    }
}
