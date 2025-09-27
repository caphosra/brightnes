use std::{io::Read, net::TcpStream, time::Instant};

use brightnes_common::serial::{APURequest, Volume};
use cpal::{
    Device, Stream, SupportedStreamConfig, default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use fundsp::{
    hacker::{AudioUnit, envelope, lfo, pulse, zero},
    net::{Net, NodeId},
};
use postcard::from_bytes;

pub struct Sound {
    _device: Device,
    _config: SupportedStreamConfig,

    pulses: [NodeId; 2],
    net: Net,
    _stream: Stream,
    start_time: Instant,
}

impl Sound {
    const MASTER_VOLUME: f32 = 0.1;

    pub fn new() -> Self {
        let device = default_host().default_output_device().unwrap();
        println!("[-] Using audio output device: {}", device.name().unwrap());

        let config = device.default_output_config().unwrap();
        let stream_config = config.config();
        let channel_count = stream_config.channels as usize;

        let mut pulse1_net = Net::new(0, 2);
        let mut pulse2_net = Net::new(0, 2);

        let pulse1 = pulse1_net.push(Box::new(zero()));
        pulse1_net.pipe_output(pulse1);

        let pulse2 = pulse2_net.push(Box::new(zero()));
        pulse2_net.pipe_output(pulse2);

        let mut net = Net::sum(pulse1_net, pulse2_net);
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

        let start_time = Instant::now();

        Self {
            _device: device,
            _config: config,
            pulses: [pulse1, pulse2],
            net,
            _stream: stream,
            start_time,
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
                let unit: Box<dyn AudioUnit> = if req.active {
                    let start = self.start_time.elapsed().as_secs_f64();
                    let end = start + req.length;

                    let pulse_state = lfo(move |t| {
                        let freq = req.frequency;
                        let freq = if req.sweep_enabled {
                            let sweep_phase = ((t - start) / req.sweep_interval).floor() as u32;
                            if req.sweep_negate {
                                freq - (freq / (1 << req.sweep_shift) as f64) * sweep_phase as f64
                            } else {
                                freq + (freq / (1 << req.sweep_shift) as f64) * sweep_phase as f64
                            }
                        } else {
                            freq
                        };
                        (freq, req.duty_rate)
                    });

                    match req.volume {
                        Volume::Constant(volume) => {
                            let wave = pulse_state
                                >> pulse()
                                    * Self::MASTER_VOLUME
                                    * (volume as f32)
                                    * envelope(move |t| if t < end { 1.0 } else { 0.0 });
                            Box::new(wave)
                        }
                        Volume::Decreasing(decreasing_time) => {
                            let wave = pulse_state
                                >> pulse()
                                    * Self::MASTER_VOLUME
                                    * envelope(move |t| {
                                        if t < end {
                                            (15 - ((t - start) / decreasing_time).floor() as u32
                                                % 15)
                                                as f64
                                                / 15.0
                                        } else {
                                            0.0
                                        }
                                    });
                            Box::new(wave)
                        }
                    }
                } else {
                    Box::new(zero())
                };
                self.net.replace(self.pulses[id], unit);
                self.net.commit();
            }
        }

        Ok(())
    }

    pub fn disable_all(&mut self) {
        for &pulse in &self.pulses {
            self.net.replace(pulse, Box::new(zero()));
        }
        self.net.commit();
    }
}
