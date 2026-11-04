//! Playing the phone's raw PCM (48 kHz stereo s16le) on the computer's default output device.
//! Packets land in a small jitter buffer that the device's audio callback drains, resampled
//! to the device's rate and mapped to its channel count.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, StreamConfig};

use crate::error::{AppError, Result};
use crate::services::scrcpy_server::{PCM_CHANNELS, PCM_RATE};

/// Buffered before playback starts (and again after an underrun), so network jitter does not crackle.
const PRIME_FRAMES: usize = PCM_RATE as usize * 40 / 1000;
/// A backlog past this is cut back to [`CATCH_UP_FRAMES`] so the sound never drifts behind the picture.
const MAX_FRAMES: usize = PCM_RATE as usize * 200 / 1000;
const CATCH_UP_FRAMES: usize = PCM_RATE as usize * 60 / 1000;
const BYTES_PER_FRAME: usize = PCM_CHANNELS as usize * 2;

/// Stereo frames waiting to be played, read out with linear-interpolation resampling.
pub struct PcmBuffer {
    frames: VecDeque<[f32; 2]>,
    /// Source frames advanced per output frame (`PCM_RATE / device rate`).
    step: f64,
    position: f64,
    primed: bool,
}

impl PcmBuffer {
    pub fn new(output_rate: u32) -> Self {
        PcmBuffer {
            frames: VecDeque::with_capacity(MAX_FRAMES),
            step: f64::from(PCM_RATE) / f64::from(output_rate.max(1)),
            position: 0.0,
            primed: false,
        }
    }

    pub fn push(&mut self, pcm: &[u8]) {
        for [l0, l1, r0, r1] in pcm.as_chunks::<BYTES_PER_FRAME>().0 {
            let left = i16::from_le_bytes([*l0, *l1]);
            let right = i16::from_le_bytes([*r0, *r1]);
            self.frames
                .push_back([f32::from_sample(left), f32::from_sample(right)]);
        }
        if self.frames.len() > MAX_FRAMES {
            let excess = self.frames.len() - CATCH_UP_FRAMES;
            self.frames.drain(..excess);
        }
    }

    /// Fill an interleaved output buffer of `channels` channels, writing silence while priming.
    pub fn fill(&mut self, out: &mut [f32], channels: usize) {
        for frame in out.chunks_mut(channels.max(1)) {
            let [left, right] = self.next_frame();
            match frame {
                [mono] => *mono = (left + right) / 2.0,
                [first, second, rest @ ..] => {
                    *first = left;
                    *second = right;
                    rest.fill(0.0);
                }
                [] => {}
            }
        }
    }

    fn next_frame(&mut self) -> [f32; 2] {
        if !self.primed {
            if self.frames.len() < PRIME_FRAMES {
                return [0.0; 2];
            }
            self.primed = true;
        }
        // Interpolation needs the frame after the current one.
        if self.frames.len() < 2 {
            self.primed = false;
            return [0.0; 2];
        }

        let (a, b) = (self.frames[0], self.frames[1]);
        let t = self.position as f32;
        let frame = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];

        self.position += self.step;
        while self.position >= 1.0 && !self.frames.is_empty() {
            self.frames.pop_front();
            self.position -= 1.0;
        }
        frame
    }
}

/// An open output stream; sound stops when it is dropped.
pub struct Player {
    _stream: cpal::Stream,
    buffer: Arc<Mutex<PcmBuffer>>,
}

impl Player {
    /// Open the default output device, at 48 kHz when it supports that.
    pub fn open() -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| AppError::Other("No audio output device on this computer".into()))?;
        let default = device
            .default_output_config()
            .map_err(|error| audio_error("read the speaker settings", error))?;
        let config = device
            .supported_output_configs()
            .ok()
            .and_then(|mut configs| {
                configs.find_map(|range| {
                    (range.sample_format() == default.sample_format())
                        .then(|| range.try_with_sample_rate(PCM_RATE))
                        .flatten()
                })
            })
            .unwrap_or(default);

        let format = config.sample_format();
        let config = config.config();
        let buffer = Arc::new(Mutex::new(PcmBuffer::new(config.sample_rate)));
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, buffer.clone()),
            SampleFormat::I16 => build::<i16>(&device, config, buffer.clone()),
            SampleFormat::U16 => build::<u16>(&device, config, buffer.clone()),
            SampleFormat::I32 => build::<i32>(&device, config, buffer.clone()),
            SampleFormat::F64 => build::<f64>(&device, config, buffer.clone()),
            other => Err(AppError::Other(format!(
                "Unsupported speaker sample format ({other})"
            ))),
        }?;
        stream
            .play()
            .map_err(|error| audio_error("start the speakers", error))?;

        Ok(Player {
            _stream: stream,
            buffer,
        })
    }

    pub fn push(&self, pcm: &[u8]) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(pcm);
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: StreamConfig,
    buffer: Arc<Mutex<PcmBuffer>>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut scratch: Vec<f32> = Vec::new();
    device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| {
                scratch.resize(data.len(), 0.0);
                match buffer.lock() {
                    Ok(mut buffer) => buffer.fill(&mut scratch, channels),
                    Err(_) => scratch.fill(0.0),
                }
                for (out, sample) in data.iter_mut().zip(&scratch) {
                    *out = T::from_sample(*sample);
                }
            },
            // A dropped device ends the sound; the phone link keeps running and can be toggled again.
            |_| {},
            None,
        )
        .map_err(|error| audio_error("open the speakers", error))
}

fn audio_error(action: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("Could not {action}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(frames: usize, left: i16, right: i16) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(frames * BYTES_PER_FRAME);
        for _ in 0..frames {
            bytes.extend_from_slice(&left.to_le_bytes());
            bytes.extend_from_slice(&right.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn stays_silent_until_primed() {
        let mut buffer = PcmBuffer::new(PCM_RATE);
        buffer.push(&pcm(PRIME_FRAMES - 1, i16::MAX, i16::MAX));
        let mut out = [1.0f32; 4];
        buffer.fill(&mut out, 2);
        assert_eq!(out, [0.0; 4]);

        buffer.push(&pcm(1, i16::MAX, i16::MAX));
        buffer.fill(&mut out, 2);
        assert!(out.iter().all(|sample| *sample > 0.99));
    }

    #[test]
    fn maps_stereo_to_mono_and_surround() {
        let mut buffer = PcmBuffer::new(PCM_RATE);
        buffer.push(&pcm(PRIME_FRAMES, i16::MAX, 0));

        let mut mono = [0.0f32; 1];
        buffer.fill(&mut mono, 1);
        assert!((mono[0] - 0.5).abs() < 0.01);

        let mut surround = [9.0f32; 6];
        buffer.fill(&mut surround, 6);
        assert!(surround[0] > 0.99);
        assert_eq!(&surround[1..], &[0.0; 5]);
    }

    #[test]
    fn drops_a_long_backlog_to_catch_up() {
        let mut buffer = PcmBuffer::new(PCM_RATE);
        buffer.push(&pcm(MAX_FRAMES + 1, 0, 0));
        assert_eq!(buffer.frames.len(), CATCH_UP_FRAMES);
    }

    #[test]
    fn resamples_to_the_device_rate() {
        let mut buffer = PcmBuffer::new(PCM_RATE / 2);
        buffer.push(&pcm(PRIME_FRAMES, 0, 0));
        let mut out = vec![0.0f32; 2 * 10];
        buffer.fill(&mut out, 2);
        // Half the device rate consumes two source frames per output frame.
        assert_eq!(buffer.frames.len(), PRIME_FRAMES - 20);
    }

    #[test]
    fn reprimes_after_an_underrun() {
        let mut buffer = PcmBuffer::new(PCM_RATE);
        buffer.push(&pcm(PRIME_FRAMES, i16::MAX, i16::MAX));
        let mut out = vec![0.0f32; 2 * PRIME_FRAMES];
        buffer.fill(&mut out, 2);
        assert!(!buffer.primed);

        buffer.push(&pcm(10, i16::MAX, i16::MAX));
        let mut next = [1.0f32; 2];
        buffer.fill(&mut next, 2);
        assert_eq!(next, [0.0; 2]);
    }
}
