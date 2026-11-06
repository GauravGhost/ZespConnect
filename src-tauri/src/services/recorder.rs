use std::borrow::Cow;
use std::io::{self, Seek, Write};
use std::time::Instant;

use crate::services::fmp4::{AudioConfig, Fmp4Writer, VideoConfig};

/// Why a recording ended before it was stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// A new picture size cannot continue the same MP4 without re-encoding.
    SizeChanged,
}

/// A stream's first timestamp, and where on the recording's timeline it plays.
#[derive(Clone, Copy)]
struct Anchor {
    pts: u64,
    at_us: u64,
}

/// The video and sound servers' timestamps share no origin, so each stream is anchored to when
/// its first packet reached the computer.
pub struct RecorderCore<W: Write + Seek> {
    out: Option<W>,
    audio_config: Option<AudioConfig>,
    writer: Option<Fmp4Writer<W>>,
    size: Option<(u32, u32)>,
    pending_config: Option<VideoConfig>,
    /// The config of a restarted encoder, waiting for its first key frame. The file header keeps
    /// the first config, so the new one goes in front of that key frame for the decoder to use.
    restart_config: Option<Vec<u8>>,
    /// When the first video frame arrived: time zero of the file.
    origin: Option<Instant>,
    video: Option<Anchor>,
    audio: Option<Anchor>,
    last_video_us: u64,
}

impl<W: Write + Seek> RecorderCore<W> {
    pub fn new(out: W, audio: Option<AudioConfig>) -> Self {
        Self {
            out: Some(out),
            audio_config: audio,
            writer: None,
            size: None,
            pending_config: None,
            restart_config: None,
            origin: None,
            video: None,
            audio: None,
            last_video_us: 0,
        }
    }

    pub fn has_audio(&self) -> bool {
        self.audio_config.is_some()
    }

    /// Feed one packet from the mirror's video stream.
    pub fn push_video(
        &mut self,
        arrived: Instant,
        config: bool,
        key: bool,
        pts: u64,
        data: &[u8],
    ) -> io::Result<Option<Ended>> {
        if config {
            return Ok(self.on_config(data));
        }
        if self.writer.is_none() {
            if !key {
                return Ok(None);
            }
            if self.pending_config.is_none() {
                return Ok(None);
            }
            let (Some(video), Some(out)) = (self.pending_config.take(), self.out.take()) else {
                return Ok(None);
            };
            self.writer = Some(Fmp4Writer::new(out, video, self.audio_config.clone())?);
            self.origin = Some(arrived);
        }
        let mut sample = Cow::Borrowed(data);
        if let Some(config) = &self.restart_config {
            // Frames from before the restart can no longer be decoded.
            if !key {
                return Ok(None);
            }
            sample = [config.as_slice(), data].concat().into();
            self.restart_config = None;
        }

        let anchor = match self.video {
            Some(anchor) => anchor,
            None => {
                // After an encoder restart the new timestamps continue from the arrival time.
                let at_us = self.offset_us(arrived).max(self.last_video_us);
                let anchor = Anchor { pts, at_us };
                self.video = Some(anchor);
                anchor
            }
        };
        let time_us = anchor.at_us + pts.saturating_sub(anchor.pts);
        self.last_video_us = time_us;
        if let Some(writer) = self.writer.as_mut() {
            writer.push_video(time_us, key, &sample)?;
        }
        Ok(None)
    }

    /// Feed one AAC frame; frames before the first video frame are dropped.
    pub fn push_audio(&mut self, arrived: Instant, pts: u64, data: &[u8]) -> io::Result<()> {
        if self.audio_config.is_none() || self.origin.is_none() {
            return Ok(());
        }
        let anchor = *self.audio.get_or_insert(Anchor {
            pts,
            at_us: self.offset_us(arrived),
        });
        let time_us = anchor.at_us + pts.saturating_sub(anchor.pts);
        match self.writer.as_mut() {
            Some(writer) => writer.push_audio(time_us, data),
            None => Ok(()),
        }
    }

    /// Close the file at `stopped`; `None` when no video frame was ever written.
    pub fn finish(self, stopped: Instant) -> io::Result<Option<W>> {
        let end_us = self.offset_us(stopped);
        self.writer
            .map(|writer| writer.finish_at(end_us))
            .transpose()
    }

    fn on_config(&mut self, data: &[u8]) -> Option<Ended> {
        // A config the phone sends that cannot be read is skipped, like the live view does.
        let Ok(video) = VideoConfig::from_annexb(data) else {
            return None;
        };
        let size = (video.width, video.height);
        if self.writer.is_some() {
            if self.size != Some(size) {
                return Some(Ended::SizeChanged);
            }
            // A new config means a restarted encoder, whose timestamps start over.
            self.video = None;
            self.restart_config = Some(data.to_vec());
            return None;
        }
        self.size = Some(size);
        self.pending_config = Some(video);
        None
    }

    fn offset_us(&self, arrived: Instant) -> u64 {
        self.origin.map_or(0, |origin| {
            arrived.saturating_duration_since(origin).as_micros() as u64
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::Duration;

    use super::*;

    const SPS_720P: [u8; 25] = [
        0x67, 0x64, 0x00, 0x1f, 0xac, 0xd9, 0x40, 0x50, 0x05, 0xbb, 0x01, 0x6c, 0x80, 0x00, 0x00,
        0x03, 0x00, 0x80, 0x00, 0x00, 0x1e, 0x07, 0x8c, 0x18, 0xcb,
    ];
    /// [`SPS_720P`] at level 4.0 instead of 3.1, as a restarted encoder may choose.
    const SPS_720P_RESTARTED: [u8; 25] = [
        0x67, 0x64, 0x00, 0x28, 0xac, 0xd9, 0x40, 0x50, 0x05, 0xbb, 0x01, 0x6c, 0x80, 0x00, 0x00,
        0x03, 0x00, 0x80, 0x00, 0x00, 0x1e, 0x07, 0x8c, 0x18, 0xcb,
    ];
    /// A 640x480 Baseline SPS.
    const SPS_480P: [u8; 9] = [0x67, 0x42, 0xC0, 0x1E, 0xDA, 0x02, 0x80, 0xF6, 0x40];
    const PPS: [u8; 4] = [0x68, 0xCE, 0x3C, 0x80];

    fn config(sps: &[u8]) -> Vec<u8> {
        [&[0, 0, 0, 1][..], sps, &[0, 0, 0, 1], &PPS].concat()
    }

    fn frame(key: bool) -> Vec<u8> {
        vec![0, 0, 0, 1, if key { 0x65 } else { 0x41 }, 0x88]
    }

    fn audio() -> AudioConfig {
        AudioConfig::from_asc(&[0x11, 0x90]).unwrap()
    }

    fn ms(base: Instant, millis: u64) -> Instant {
        base + Duration::from_millis(millis)
    }

    fn count(file: &[u8], kind: &[u8; 4]) -> usize {
        file.windows(4).filter(|window| window == kind).count()
    }

    /// The `tfdt` decode times of each fragment of `track`, in order.
    fn decode_times(file: &[u8], track: u32) -> Vec<u64> {
        let mut times = Vec::new();
        for at in 0..file.len().saturating_sub(4) {
            if &file[at..at + 4] == b"tfhd" && file[at + 8..at + 12] == track.to_be_bytes() {
                let tfdt = at + 12 + 4;
                assert_eq!(&file[tfdt..tfdt + 4], b"tfdt");
                times.push(u64::from_be_bytes(
                    file[tfdt + 8..tfdt + 16].try_into().unwrap(),
                ));
            }
        }
        times
    }

    #[test]
    fn waits_for_config_and_key() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        core.push_video(base, false, false, 0, &frame(false))
            .unwrap();
        core.push_video(base, false, true, 0, &frame(true)).unwrap();
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, false, 10, &frame(false))
            .unwrap();
        // Nothing usable yet: a delta before config, a key before config, a delta after.
        assert!(RecorderCore::new(Cursor::new(Vec::new()), None)
            .finish(base)
            .unwrap()
            .is_none());

        core.push_video(ms(base, 5), false, true, 20, &frame(true))
            .unwrap();
        let file = core.finish(base).unwrap().expect("a file").into_inner();
        assert_eq!(count(&file, b"moof"), 1);
        assert_eq!(decode_times(&file, 1), [0]);
    }

    #[test]
    fn same_size_config_continues() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 0, &frame(true)).unwrap();
        let again = core
            .push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        assert_eq!(again, None);
    }

    #[test]
    fn size_change_ends() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 0, &frame(true)).unwrap();
        let changed = core
            .push_video(base, true, false, 0, &config(&SPS_480P))
            .unwrap();
        assert_eq!(changed, Some(Ended::SizeChanged));
    }

    #[test]
    fn aligns_streams_by_arrival() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), Some(audio()));
        assert!(core.has_audio());
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 5_000_000, &frame(true))
            .unwrap();
        core.push_audio(ms(base, 40), 900_000, &[0x21]).unwrap();
        core.push_audio(ms(base, 61), 921_333, &[0x21]).unwrap();
        let file = core.finish(base).unwrap().unwrap().into_inner();
        assert_eq!(decode_times(&file, 1), [0]);
        // 40 ms at 48 kHz.
        assert_eq!(decode_times(&file, 2), [1920]);
    }

    #[test]
    fn drops_audio_before_video() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), Some(audio()));
        core.push_audio(base, 100, &[0x21]).unwrap();
        core.push_video(ms(base, 10), true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(ms(base, 10), false, true, 0, &frame(true))
            .unwrap();
        let file = core.finish(base).unwrap().unwrap().into_inner();
        assert!(decode_times(&file, 2).is_empty());
    }

    #[test]
    fn reanchors_after_encoder_reset() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 9_000_000, &frame(true))
            .unwrap();
        core.push_video(ms(base, 33), false, false, 9_033_333, &frame(false))
            .unwrap();
        // RESET_VIDEO restarts the encoder, whose timestamps start over.
        let restarted = config(&SPS_720P_RESTARTED);
        core.push_video(ms(base, 500), true, false, 0, &restarted)
            .unwrap();
        // A frame that was still on its way from the old encoder.
        core.push_video(ms(base, 500), false, false, 9_066_666, &frame(false))
            .unwrap();
        core.push_video(ms(base, 500), false, true, 0, &frame(true))
            .unwrap();
        core.push_video(ms(base, 533), false, false, 33_333, &frame(false))
            .unwrap();
        let file = core.finish(base).unwrap().unwrap().into_inner();
        // One fragment; the samples after the reset keep moving forward from 500 ms.
        let trun = file
            .windows(4)
            .position(|window| window == b"trun")
            .unwrap();
        let duration = |index: usize| {
            let at = trun + 4 + 4 + 8 + index * 12;
            u32::from_be_bytes(file[at..at + 4].try_into().unwrap())
        };
        let size = |index: usize| {
            let at = trun + 4 + 4 + 8 + index * 12 + 4;
            u32::from_be_bytes(file[at..at + 4].try_into().unwrap())
        };
        assert_eq!(
            u32::from_be_bytes(file[trun + 8..trun + 12].try_into().unwrap()),
            4
        );
        assert_eq!(duration(0), 3000);
        // 33 ms to 500 ms at 90 kHz.
        assert_eq!(duration(1), 42_000);
        assert_eq!(duration(2), 3000);
        // The restarted key frame carries the new SPS and PPS: three 4-byte lengths + NALs.
        let frame_len = frame(true).len() - 4;
        assert_eq!(
            size(2) as usize,
            12 + SPS_720P_RESTARTED.len() + PPS.len() + frame_len
        );
        let sps_at = file
            .windows(SPS_720P_RESTARTED.len())
            .position(|window| window == SPS_720P_RESTARTED)
            .expect("new SPS in the samples");
        assert!(sps_at > trun);
    }

    #[test]
    fn last_frame_lasts_until_stop() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 0, &frame(true)).unwrap();
        // Nothing changes on screen for 30 s, so nothing more arrives.
        let file = core
            .finish(base + Duration::from_secs(30))
            .unwrap()
            .unwrap()
            .into_inner();
        let trun = file
            .windows(4)
            .position(|window| window == b"trun")
            .unwrap();
        let at = trun + 4 + 4 + 8;
        let duration = u32::from_be_bytes(file[at..at + 4].try_into().unwrap());
        assert_eq!(duration, 30 * 90_000);
    }

    #[test]
    fn video_only_when_audio_fails() {
        let base = Instant::now();
        let mut core = RecorderCore::new(Cursor::new(Vec::new()), None);
        assert!(!core.has_audio());
        core.push_video(base, true, false, 0, &config(&SPS_720P))
            .unwrap();
        core.push_video(base, false, true, 0, &frame(true)).unwrap();
        core.push_audio(ms(base, 5), 0, &[0x21]).unwrap();
        let file = core.finish(base).unwrap().unwrap().into_inner();
        assert_eq!(count(&file, b"trak"), 1);
    }

    /// Mux a packet dump captured from a real phone (`ZESP_DUMP`) into `ZESP_DUMP_OUT`.
    /// Record: `[u8 stream][u64 arrived_us][u8 config][u8 key][u64 pts][u32 len][data]`.
    #[test]
    #[ignore]
    fn muxes_a_phone_dump() {
        let dump = std::fs::read(std::env::var("ZESP_DUMP").unwrap()).unwrap();
        let base = Instant::now();
        let mut packets = Vec::new();
        let mut at = 0;
        while at + 23 <= dump.len() {
            let stream = dump[at];
            let arrived = u64::from_be_bytes(dump[at + 1..at + 9].try_into().unwrap());
            let config = dump[at + 9] == 1;
            let key = dump[at + 10] == 1;
            let pts = u64::from_be_bytes(dump[at + 11..at + 19].try_into().unwrap());
            let len = u32::from_be_bytes(dump[at + 19..at + 23].try_into().unwrap()) as usize;
            let data = dump[at + 23..at + 23 + len].to_vec();
            packets.push((
                stream,
                base + Duration::from_micros(arrived),
                config,
                key,
                pts,
                data,
            ));
            at += 23 + len;
        }
        let asc = packets
            .iter()
            .find(|p| p.0 == 1 && p.2)
            .map(|p| p.5.clone());
        let mut core = RecorderCore::new(
            Cursor::new(Vec::new()),
            asc.map(|asc| AudioConfig::from_asc(&asc).unwrap()),
        );
        for (stream, arrived, config, key, pts, data) in packets {
            if stream == 0 {
                assert_eq!(
                    core.push_video(arrived, config, key, pts, &data).unwrap(),
                    None
                );
            } else if !config {
                core.push_audio(arrived, pts, &data).unwrap();
            }
        }
        let file = core
            .finish(base)
            .unwrap()
            .expect("video was recorded")
            .into_inner();
        std::fs::write(std::env::var("ZESP_DUMP_OUT").unwrap(), file).unwrap();
    }
}
