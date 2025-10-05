use core::ptr::{read_volatile, write_volatile};

use crate::{
    drivers::virtio::{VirtIODevice, VirtQ, VirtQDesc, VIRT_QUEUE_SIZE},
    error, info,
    mem::MemoryAllocator,
};

#[repr(C)]
struct SoundConfig {
    jacks: u32,
    streams: u32,
    chmaps: u32,
    controls: u32,
}

#[repr(C)]
struct SoundPCMHeader {
    code: SoundRequestType,
    stream_id: u32,
}

/// Original: `struct virtio_snd_pcm_xfer`
#[repr(C)]
struct SoundPCMTransferHeader {
    stream_id: u32,
}

#[repr(u32)]
enum SoundRequestType {
    PCMInfo = 0x100,
    PCMSetParams,
    PCMPrepare,
    PCMRelease,
    PCMStart,
    PCMStop,
}

#[repr(u32)]
enum SoundStatus {
    OK = 0x8000,
    BadMessage,
    NotSupported,
    IOError,
}

/// Original: `VIRTIO_SND_PCM_FMT_*`
#[repr(u8)]
enum SoundPCMFormat {
    IMAADPCM = 0, /*  4 /  4 bits */
    MuLaw,        /*  8 /  8 bits */
    ALaw,         /*  8 /  8 bits */
    S8,           /*  8 /  8 bits */
    U8,           /*  8 /  8 bits */
    S16,          /* 16 / 16 bits */
    U16,          /* 16 / 16 bits */
}

/// Original: `VIRTIO_SND_PCM_RATE_*`
#[repr(u8)]
enum SoundPCMRate {
    Rate48000 = 7,
}

#[repr(C)]
struct SoundPCMSetParams {
    header: SoundPCMHeader,
    buffer_bytes: u32,
    period_bytes: u32,
    features: u32,
    channels: u8,
    format: SoundPCMFormat,
    rate: SoundPCMRate,
    padding: u8,
}

/// Original: `struct virtio_snd_pcm_status`
#[repr(C)]
struct SoundPCMStatus {
    status: SoundStatus,
    latency_bytes: u32,
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

    tx_dummy_status: SoundPCMStatus,

    config: &'a mut SoundConfig,
}

impl<'a> VirtSoundDevice<'a> {
    pub const VIRTIO_SOUND_DEVICE_ID: u16 = 0x1059;

    pub const PERIOD_BYTES: u32 = 0x1000;
    const MAX_TX_REQUESTS: usize = VIRT_QUEUE_SIZE / 3;

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

        info!(DRV, "Num of sound streams: {}", config.streams);

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
            tx_dummy_status: SoundPCMStatus {
                status: SoundStatus::IOError,
                latency_bytes: 0,
            },
            config,
        })
    }

    fn control_request<T>(&mut self, query: &T) {
        // Write the request to the descriptor table.
        unsafe {
            write_volatile(
                &mut self.control_queue.desc[0].addr,
                query as *const _ as u64,
            );
            write_volatile(&mut self.control_queue.desc[0].len, size_of::<T>() as u32);
            write_volatile(&mut self.control_queue.desc[0].flags, VirtQDesc::F_NEXT);
            write_volatile(&mut self.control_queue.desc[0].next, 1);
        }

        // Write the status byte to the descriptor table.
        let mut status: SoundStatus = SoundStatus::IOError;
        unsafe {
            write_volatile(
                &mut self.control_queue.desc[1].addr,
                &status as *const _ as u64,
            );
            write_volatile(&mut self.control_queue.desc[1].len, size_of::<u32>() as u32);
            write_volatile(&mut self.control_queue.desc[1].flags, VirtQDesc::F_WRITE);
            write_volatile(&mut self.control_queue.desc[1].next, 0);
        }

        self.control_queue.push(0, self.control_queue_notify_addr);

        loop {
            // Wait until the device processes the request.
            let used_idx = unsafe { read_volatile(&mut self.control_queue.used.idx) };
            if used_idx != self.control_queue.last_used_idx {
                self.control_queue.last_used_idx = used_idx;
                break;
            }
        }

        let status = unsafe { read_volatile(&mut status) };
        match status {
            SoundStatus::OK => {
                info!(DRV, "Sound device request completed.");
            }
            SoundStatus::BadMessage => {
                error!(DRV, "Sound device request failed: BadMessage");
            }
            SoundStatus::NotSupported => {
                error!(DRV, "Sound device request failed: NotSupported");
            }
            SoundStatus::IOError => {
                error!(DRV, "Sound device request failed: IOError");
            }
        }
    }

    pub fn prepare(&mut self) -> Result<(), ()> {
        let req = SoundPCMHeader {
            code: SoundRequestType::PCMPrepare,
            stream_id: 0,
        };
        self.control_request(&req);
        Ok(())
    }

    pub fn set_params(&mut self) -> Result<(), ()> {
        let req = SoundPCMSetParams {
            header: SoundPCMHeader {
                code: SoundRequestType::PCMSetParams,
                stream_id: 0,
            },
            buffer_bytes: Self::PERIOD_BYTES * (size_of::<i16>() * 2) as u32,
            period_bytes: Self::PERIOD_BYTES * (size_of::<i16>() * 2) as u32,
            features: 0,
            channels: 2,
            format: SoundPCMFormat::S16,
            rate: SoundPCMRate::Rate48000,
            padding: 0,
        };
        self.control_request(&req);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), ()> {
        let req = SoundPCMHeader {
            code: SoundRequestType::PCMStart,
            stream_id: 0,
        };
        self.control_request(&req);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), ()> {
        let req = SoundPCMHeader {
            code: SoundRequestType::PCMStop,
            stream_id: 0,
        };
        self.control_request(&req);
        Ok(())
    }

    pub fn tx_consumed_count(&self) -> u16 {
        unsafe { read_volatile(&self.tx_queue.used.idx) }
    }

    pub fn write_stream(&mut self, index: u16, buf: &[u8]) {
        let query = SoundPCMTransferHeader { stream_id: 0 };

        // Write the request to the descriptor table.
        unsafe {
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3].addr,
                &query as *const _ as u64,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3].len,
                size_of::<SoundPCMTransferHeader>() as u32,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3].flags,
                VirtQDesc::F_NEXT,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3].next,
                index as u16 * 3 + 1,
            );
        }

        // Write the buffer to the descriptor table.
        unsafe {
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 1].addr,
                buf.as_ptr() as u64,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 1].len,
                buf.len() as u32,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 1].flags,
                VirtQDesc::F_NEXT,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 1].next,
                index as u16 * 3 + 2,
            );
        }

        // Write the status byte to the descriptor table.
        unsafe {
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 2].addr,
                &self.tx_dummy_status as *const _ as u64,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 2].len,
                size_of::<SoundPCMStatus>() as u32,
            );
            write_volatile(
                &mut self.tx_queue.desc[index as usize * 3 + 2].flags,
                VirtQDesc::F_WRITE,
            );
            write_volatile(&mut self.tx_queue.desc[index as usize * 3 + 2].next, 0);
        }

        self.tx_queue
            .push(index as u16 * 3, self.tx_queue_notify_addr);
    }
}
