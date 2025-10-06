use std::{io::Read, net::TcpStream};

use brightnes_common::serial::{APURequest, PulseRequest, TriangleRequest};
use cpal::{
    Device, Stream, SupportedStreamConfig, default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use fundsp::{
    hacker::{AudioUnit, Fade, constant, lfo, pulse, triangle, zero},
    net::{Net, NodeId},
};
use postcard::from_bytes;

pub struct Sound {
    _device: Device,
    _config: SupportedStreamConfig,

    last_pulses_req: [Option<PulseRequest>; 2],
    last_triangle_req: Option<TriangleRequest>,

    output: NodeId,
    net: Net,
    _stream: Stream,
}

impl Sound {
    const MASTER_VOLUME: f32 = 0.1;
    const FADE_TIME: f32 = 0.001;

    pub const CPU_CLOCK_FREQUENCY: f64 = 1789773.0;
    pub const TRIANGLE_FREQUENCY_LIMIT: f64 = Self::CPU_CLOCK_FREQUENCY / 32.0 / 2.0;

    pub fn new() -> Self {
        let device = default_host().default_output_device().unwrap();
        println!("[-] Using audio output device: {}", device.name().unwrap());

        let config = device.default_output_config().unwrap();
        let stream_config = config.config();
        let channel_count = stream_config.channels as usize;

        let mut net = Net::new(0, 2);

        let output = net.push(Box::new(zero()));
        net.pipe_output(output);

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
            last_pulses_req: [None, None],
            last_triangle_req: None,
            output,
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
                self.last_pulses_req[id] = Some(req);
            }
            APURequest::Triangle(req) => {
                self.last_triangle_req = Some(req);
            }
        }

        let output_wave = zero();

        macro_rules! add_pulse_wave {
            ($wave:expr, $id:expr) => {{
                let (frequency, duty_rate, volume) =
                    if let Some(last_req) = &self.last_pulses_req[$id] {
                        if last_req.active {
                            (
                                last_req.frequency,
                                last_req.duty_rate,
                                last_req.volume as f32,
                            )
                        } else {
                            (0.0, 0.0, 0.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0)
                    };
                $wave
                    + (lfo(move |_t| (frequency, duty_rate))
                        >> pulse() * Self::MASTER_VOLUME * volume)
            }};
        }

        // Pulse waves
        let output_wave = add_pulse_wave!(output_wave, 0);
        let output_wave = add_pulse_wave!(output_wave, 1);

        // Triangle wave
        let frequency = if let Some(last_req) = &self.last_triangle_req {
            if last_req.active {
                last_req.frequency
            } else {
                0.0
            }
        } else {
            0.0
        };
        let output_wave =
            output_wave + (constant(frequency as f32) >> triangle() * Self::MASTER_VOLUME);

        self.net.crossfade(
            self.output,
            Fade::Smooth,
            Self::FADE_TIME,
            Box::new(output_wave),
        );
        self.net.commit();

        Ok(())
    }

    pub fn disable_all(&mut self) {
        self.net
            .crossfade(self.output, Fade::Smooth, Self::FADE_TIME, Box::new(zero()));
        self.net.commit();
    }
}
