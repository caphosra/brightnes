use std::{io::Read, net::TcpStream};

use brightnes_common::serial::APURequest;
use cpal::{
    Device, Stream, SupportedStreamConfig, default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use fundsp::{
    hacker::{AudioUnit, Fade, envelope, lfo, pulse, zero},
    hacker32::{constant, triangle},
    net::{Net, NodeId},
};
use postcard::from_bytes;

pub struct Sound {
    _device: Device,
    _config: SupportedStreamConfig,

    pulses: [NodeId; 2],
    triangle: NodeId,
    net: Net,
    _stream: Stream,
}

impl Sound {
    const MASTER_VOLUME: f32 = 0.1;
    const FADE_TIME: f32 = 0.00001;

    pub const CPU_CLOCK_FREQUENCY: f64 = 1789773.0;
    pub const TRIANGLE_FREQUENCY_LIMIT: f64 = Self::CPU_CLOCK_FREQUENCY / 32.0 / 2.0;

    pub fn new() -> Self {
        let device = default_host().default_output_device().unwrap();
        println!("[-] Using audio output device: {}", device.name().unwrap());

        let config = device.default_output_config().unwrap();
        let stream_config = config.config();
        let channel_count = stream_config.channels as usize;

        let mut pulse1_net = Net::new(0, 2);
        let mut pulse2_net = Net::new(0, 2);
        let mut triangle_net = Net::new(0, 2);

        let pulse1 = pulse1_net.push(Box::new(zero()));
        pulse1_net.pipe_output(pulse1);
        println!("[-] Pulse 1 node ID: {}", pulse1.value());

        let pulse2 = pulse2_net.push(Box::new(zero()));
        pulse2_net.pipe_output(pulse2);
        println!("[-] Pulse 2 node ID: {}", pulse2.value());

        let triangle = triangle_net.push(Box::new(zero()));
        triangle_net.pipe_output(triangle);
        println!("[-] Triangle node ID: {}", triangle.value());

        let pulse_nets = Net::sum(pulse1_net, pulse2_net);
        let mut net = Net::sum(pulse_nets, triangle_net);
        net.set_sample_rate(config.sample_rate().0 as f64);

        let mut backend = net.backend();

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    for chunk in data.chunks_mut(channel_count) {
                        let (left, right) = backend.get_stereo();
                        for (channel, sample) in chunk.iter_mut().enumerate() {
                            if channel & 1 == 0 {
                                *sample = left;
                            } else {
                                *sample = right;
                            }
                        }
                    }
                    ()
                },
                move |err| println!("[!] Streaming error: {}", err),
                None,
            )
            .unwrap();
        stream.play().unwrap();

        Self {
            _device: device,
            _config: config,
            pulses: [pulse1, pulse2],
            triangle,
            net,
            _stream: stream,
        }
    }

    pub fn receive_request(&mut self, stream: &mut TcpStream) -> std::io::Result<()> {
        let mut size_buf = [0u8; 4];
        stream.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut data_buf = vec![0u8; size];
        stream.read_exact(&mut data_buf)?;

        let request = from_bytes(&data_buf).map_err(|_| {
            println!("[!] Failed to deserialize APU request.");
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Deserialization error")
        })?;

        match request {
            APURequest::Pulse(id, req) => {
                println!(
                    "[-] Pulse request: id={} active={}, frequency={}, volume={}, duty_rate={}",
                    id, req.active, req.frequency as f32, req.volume as f32, req.duty_rate as f32
                );
                let unit: Box<dyn AudioUnit> = if req.active {
                    let wave = lfo(move |_t| (req.frequency, req.duty_rate))
                        >> pulse() * Self::MASTER_VOLUME * (req.volume as f32);
                    Box::new(wave)
                } else {
                    Box::new(zero())
                };
                self.net.replace(self.pulses[id], unit);
                self.net.commit();
            }
            APURequest::Triangle(req) => {
                let unit: Box<dyn AudioUnit> =
                    if req.active && req.frequency < Self::TRIANGLE_FREQUENCY_LIMIT {
                        let wave = constant(req.frequency as f32)
                            >> triangle()
                                * Self::MASTER_VOLUME
                                * envelope(move |t| if t < req.length { 1.0 } else { 0.0 });
                        Box::new(wave)
                    } else {
                        Box::new(zero())
                    };
                self.net
                    .crossfade(self.triangle, Fade::Smooth, Self::FADE_TIME, unit);
                self.net.commit();
            }
        }

        Ok(())
    }

    pub fn disable_all(&mut self) {
        for &pulse in &self.pulses {
            self.net
                .crossfade(pulse, Fade::Smooth, Self::FADE_TIME, Box::new(zero()));
        }
        self.net.crossfade(
            self.triangle,
            Fade::Smooth,
            Self::FADE_TIME,
            Box::new(zero()),
        );
        self.net.commit();
    }
}
