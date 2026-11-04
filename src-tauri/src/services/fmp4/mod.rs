//! A small fragmented-MP4 writer for recordings: H.264 video as the phone encoded it, plus
//! optional AAC sound, with no re-encoding. Every flushed fragment is complete on disk, so a
//! file cut short by a crash or a lost phone still plays up to its last fragment.

mod annexb;
mod boxes;

use std::io::{self, Write};

use crate::error::{AppError, Result};
use boxes::{Sample, AUDIO_TRACK, NON_SYNC_SAMPLE, SYNC_SAMPLE, VIDEO_TIMESCALE, VIDEO_TRACK};

/// How much media a track buffers before it is written out as a fragment.
const FRAGMENT_US: u64 = 1_000_000;
/// The last sample's duration when nothing came before it to measure one from.
const FALLBACK_VIDEO_US: u64 = 33_333;
const AAC_FRAME_SAMPLES: u64 = 1024;
const AAC_SAMPLE_RATES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoConfig {
    pub sps: Vec<u8>,
    pub pps: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl VideoConfig {
    /// Read the SPS/PPS and picture size from a codec-config packet.
    pub fn from_annexb(config: &[u8]) -> Result<Self> {
        use h264_reader::nal::sps::SeqParameterSet;
        use h264_reader::rbsp::{decode_nal, BitReader};

        let invalid = |what: &str| AppError::Parse(format!("H.264 config: {what}"));
        let (sps, pps) = annexb::parameter_sets(config).ok_or_else(|| invalid("no SPS and PPS"))?;
        if sps.len() < 4 {
            return Err(invalid("SPS too short"));
        }
        let rbsp = decode_nal(sps).map_err(|error| invalid(&error.to_string()))?;
        let (width, height) = SeqParameterSet::from_bits(BitReader::new(&*rbsp))
            .and_then(|parsed| parsed.pixel_dimensions())
            .map_err(|error| invalid(&format!("{error:?}")))?;
        Ok(Self {
            sps: sps.to_vec(),
            pps: pps.to_vec(),
            width,
            height,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioConfig {
    pub asc: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioConfig {
    /// Read the rate and channels from an AAC AudioSpecificConfig.
    pub fn from_asc(asc: &[u8]) -> Result<Self> {
        let invalid = || AppError::Parse("AAC config is invalid".into());
        let [first, second, ..] = asc else {
            return Err(invalid());
        };
        // 5 bits object type, 4 bits sampling frequency index, 4 bits channel configuration.
        let rate_index = usize::from(((first & 0x07) << 1) | (second >> 7));
        let channels = u16::from((second >> 3) & 0x0F);
        let sample_rate = *AAC_SAMPLE_RATES.get(rate_index).ok_or_else(invalid)?;
        if channels == 0 {
            return Err(invalid());
        }
        Ok(Self {
            asc: asc.to_vec(),
            sample_rate,
            channels,
        })
    }
}

fn to_ticks(time_us: u64, timescale: u32) -> u64 {
    ((u128::from(time_us) * u128::from(timescale) + 500_000) / 1_000_000) as u64
}

/// A sample whose duration is known once the next one arrives.
struct Held {
    time: u64,
    flags: u32,
    data: Vec<u8>,
}

struct Track {
    id: u32,
    timescale: u32,
    held: Option<Held>,
    samples: Vec<Sample>,
    /// Decode time of `samples[0]`.
    start: u64,
    span: u64,
    last_duration: u32,
    fallback_duration: u32,
}

impl Track {
    fn new(id: u32, timescale: u32, fallback_duration: u32) -> Self {
        Self {
            id,
            timescale,
            held: None,
            samples: Vec::new(),
            start: 0,
            span: 0,
            last_duration: 0,
            fallback_duration,
        }
    }

    /// Queue a sample; true when enough is buffered to write a fragment.
    fn push(&mut self, time_us: u64, flags: u32, data: Vec<u8>) -> bool {
        let mut time = to_ticks(time_us, self.timescale);
        if let Some(previous) = self.held.take() {
            // Timestamps only move forward, or a duration would be zero or negative.
            time = time.max(previous.time + 1);
            let duration = (time - previous.time) as u32;
            self.complete(previous, duration);
        }
        self.held = Some(Held { time, flags, data });
        self.span >= to_ticks(FRAGMENT_US, self.timescale)
    }

    fn complete(&mut self, sample: Held, duration: u32) {
        if self.samples.is_empty() {
            self.start = sample.time;
        }
        self.span += u64::from(duration);
        self.last_duration = duration;
        self.samples.push(Sample {
            duration,
            flags: sample.flags,
            data: sample.data,
        });
    }

    fn release_held(&mut self) {
        if let Some(last) = self.held.take() {
            let duration = if self.last_duration > 0 {
                self.last_duration
            } else {
                self.fallback_duration
            };
            self.complete(last, duration);
        }
    }

    fn take_fragment(&mut self, sequence: u32) -> Option<Vec<u8>> {
        if self.samples.is_empty() {
            return None;
        }
        let fragment = boxes::fragment(sequence, self.id, self.start, &self.samples);
        self.samples.clear();
        self.span = 0;
        Some(fragment)
    }
}

pub struct Fmp4Writer<W: Write> {
    out: W,
    sequence: u32,
    video: Track,
    audio: Option<Track>,
}

impl<W: Write> Fmp4Writer<W> {
    /// Write the file header (`ftyp` + `moov`).
    pub fn new(mut out: W, video: VideoConfig, audio: Option<AudioConfig>) -> io::Result<Self> {
        out.write_all(&boxes::ftyp())?;
        out.write_all(&boxes::moov(&video, audio.as_ref()))?;
        out.flush()?;
        let video_fallback = to_ticks(FALLBACK_VIDEO_US, VIDEO_TIMESCALE) as u32;
        Ok(Self {
            out,
            sequence: 0,
            video: Track::new(VIDEO_TRACK, VIDEO_TIMESCALE, video_fallback),
            audio: audio.map(|config| {
                Track::new(AUDIO_TRACK, config.sample_rate, AAC_FRAME_SAMPLES as u32)
            }),
        })
    }

    /// Add one Annex-B access unit at `time_us` on the recording's timeline.
    pub fn push_video(&mut self, time_us: u64, key: bool, annexb: &[u8]) -> io::Result<()> {
        let flags = if key { SYNC_SAMPLE } else { NON_SYNC_SAMPLE };
        if self.video.push(time_us, flags, annexb::to_avcc(annexb)) {
            self.write_video()?;
        }
        Ok(())
    }

    /// Add one raw AAC frame at `time_us` on the recording's timeline. Ignored without an
    /// audio track.
    pub fn push_audio(&mut self, time_us: u64, frame: &[u8]) -> io::Result<()> {
        let Some(track) = self.audio.as_mut() else {
            return Ok(());
        };
        if track.push(time_us, SYNC_SAMPLE, frame.to_vec()) {
            self.write_audio()?;
        }
        Ok(())
    }

    /// Write out everything still buffered and hand back the output.
    pub fn finish(mut self) -> io::Result<W> {
        self.video.release_held();
        self.write_video()?;
        if let Some(track) = self.audio.as_mut() {
            track.release_held();
        }
        self.write_audio()?;
        Ok(self.out)
    }

    fn write_video(&mut self) -> io::Result<()> {
        if let Some(fragment) = self.video.take_fragment(self.sequence + 1) {
            self.write(fragment)?;
        }
        Ok(())
    }

    fn write_audio(&mut self) -> io::Result<()> {
        let sequence = self.sequence + 1;
        if let Some(fragment) = self
            .audio
            .as_mut()
            .and_then(|track| track.take_fragment(sequence))
        {
            self.write(fragment)?;
        }
        Ok(())
    }

    fn write(&mut self, fragment: Vec<u8>) -> io::Result<()> {
        self.sequence += 1;
        self.out.write_all(&fragment)?;
        // Each fragment reaches the disk whole, so a cut-short file still plays.
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1280x720 High profile SPS (from h264-reader's own test cases).
    const SPS_720P: [u8; 25] = [
        0x67, 0x64, 0x00, 0x1f, 0xac, 0xd9, 0x40, 0x50, 0x05, 0xbb, 0x01, 0x6c, 0x80, 0x00, 0x00,
        0x03, 0x00, 0x80, 0x00, 0x00, 0x1e, 0x07, 0x8c, 0x18, 0xcb,
    ];
    /// AAC-LC, 48 kHz, stereo.
    const ASC_48K_STEREO: [u8; 2] = [0x11, 0x90];

    fn config_packet() -> Vec<u8> {
        [
            &[0, 0, 0, 1][..],
            &SPS_720P,
            &[0, 0, 0, 1, 0x68, 0xEB, 0xE3, 0xCB, 0x22, 0xC0],
        ]
        .concat()
    }

    fn video() -> VideoConfig {
        VideoConfig::from_annexb(&config_packet()).unwrap()
    }

    fn audio() -> AudioConfig {
        AudioConfig::from_asc(&ASC_48K_STEREO).unwrap()
    }

    fn frame(key: bool) -> Vec<u8> {
        vec![0, 0, 0, 1, if key { 0x65 } else { 0x41 }, 0x88, 0x84]
    }

    /// `(fourcc, payload)` for each box in `data`.
    fn walk(data: &[u8]) -> Vec<(String, &[u8])> {
        let mut boxes = Vec::new();
        let mut at = 0;
        while at + 8 <= data.len() {
            let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
            assert!(
                size >= 8 && at + size <= data.len(),
                "box overruns its parent"
            );
            let kind = String::from_utf8_lossy(&data[at + 4..at + 8]).into_owned();
            boxes.push((kind, &data[at + 8..at + size]));
            at += size;
        }
        assert_eq!(at, data.len(), "trailing bytes after the last box");
        boxes
    }

    fn child<'a>(payload: &'a [u8], kind: &str) -> &'a [u8] {
        walk(payload)
            .into_iter()
            .find(|(name, _)| name == kind)
            .unwrap()
            .1
    }

    fn u32_at(data: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(data[at..at + 4].try_into().unwrap())
    }

    /// Per-sample `(duration, size, flags)` of a `moof`'s `trun`.
    fn trun_samples(moof: &[u8]) -> Vec<(u32, u32, u32)> {
        let trun = child(child(moof, "traf"), "trun");
        let count = u32_at(trun, 4) as usize;
        (0..count)
            .map(|index| {
                let at = 12 + index * 12;
                (u32_at(trun, at), u32_at(trun, at + 4), u32_at(trun, at + 8))
            })
            .collect()
    }

    fn record(seconds: u64, with_audio: bool) -> Vec<u8> {
        let mut writer = Fmp4Writer::new(Vec::new(), video(), with_audio.then(audio)).unwrap();
        for index in 0..seconds * 30 {
            writer
                .push_video(index * 33_333, index % 30 == 0, &frame(index % 30 == 0))
                .unwrap();
        }
        writer.finish().unwrap()
    }

    #[test]
    fn sps_dimensions() {
        let config = video();
        assert_eq!((config.width, config.height), (1280, 720));
        assert_eq!(config.sps, SPS_720P);
    }

    #[test]
    fn reads_audio_specific_config() {
        assert_eq!((audio().sample_rate, audio().channels), (48_000, 2));
        assert!(AudioConfig::from_asc(&[0x11]).is_err());
    }

    #[test]
    fn header_layout() {
        let file = Fmp4Writer::new(Vec::new(), video(), Some(audio()))
            .unwrap()
            .finish()
            .unwrap();
        let top: Vec<_> = walk(&file).into_iter().map(|(kind, _)| kind).collect();
        assert_eq!(top, ["ftyp", "moov"]);
        let traks = |file: &[u8]| {
            walk(child(file, "moov"))
                .into_iter()
                .filter(|(kind, _)| kind == "trak")
                .count()
        };
        assert_eq!(traks(&file), 2);
        let silent = Fmp4Writer::new(Vec::new(), video(), None)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(traks(&silent), 1);
    }

    #[test]
    fn fragments_increment_sequence() {
        let file = record(3, false);
        let sequences: Vec<u32> = walk(&file)
            .into_iter()
            .filter(|(kind, _)| kind == "moof")
            .map(|(_, moof)| u32_at(child(moof, "mfhd"), 4))
            .collect();
        assert!(sequences.len() >= 3, "{sequences:?}");
        assert_eq!(sequences, (1..=sequences.len() as u32).collect::<Vec<_>>());
    }

    #[test]
    fn key_frames_marked_sync() {
        let file = record(2, false);
        let (_, moof) = walk(&file)
            .into_iter()
            .find(|(kind, _)| kind == "moof")
            .unwrap();
        let samples = trun_samples(moof);
        assert_eq!(samples[0].2, SYNC_SAMPLE);
        assert_eq!(samples[1].2, NON_SYNC_SAMPLE);
        // 33_333 µs at 90 kHz, rounded.
        assert_eq!(samples[0].0, 3000);
        // AVCC: a 4-byte length replaced the 4-byte start code.
        assert_eq!(samples[0].1, 7);
    }

    #[test]
    fn single_sample_fragment_has_duration() {
        let mut writer = Fmp4Writer::new(Vec::new(), video(), None).unwrap();
        writer.push_video(0, true, &frame(true)).unwrap();
        let file = writer.finish().unwrap();
        let (_, moof) = walk(&file)
            .into_iter()
            .find(|(kind, _)| kind == "moof")
            .unwrap();
        let samples = trun_samples(moof);
        assert_eq!(samples.len(), 1);
        assert!(samples[0].0 > 0);
    }

    #[test]
    fn audio_goes_to_its_own_track() {
        let mut writer = Fmp4Writer::new(Vec::new(), video(), Some(audio())).unwrap();
        writer.push_video(0, true, &frame(true)).unwrap();
        for index in 0..60 {
            writer
                .push_audio(40_000 + index * 21_333, &[0x21, 0x10, 0x04])
                .unwrap();
        }
        let file = writer.finish().unwrap();
        let audio_moof = walk(&file)
            .into_iter()
            .filter(|(kind, _)| kind == "moof")
            .map(|(_, moof)| moof)
            .find(|moof| u32_at(child(child(moof, "traf"), "tfhd"), 4) == AUDIO_TRACK)
            .expect("an audio fragment");
        // The first audio sample starts 40 ms in: 1920 samples at 48 kHz.
        let tfdt = child(child(audio_moof, "traf"), "tfdt");
        assert_eq!(u64::from_be_bytes(tfdt[4..12].try_into().unwrap()), 1920);
        assert_eq!(trun_samples(audio_moof)[0].0, 1024);
    }

    #[test]
    fn truncated_file_still_parses() {
        let mut file = Vec::new();
        {
            let mut writer = Fmp4Writer::new(&mut file, video(), None).unwrap();
            for index in 0..60 {
                writer
                    .push_video(index * 33_333, index % 30 == 0, &frame(index % 30 == 0))
                    .unwrap();
            }
            // Dropped without `finish`, like a crash.
        }
        let boxes = walk(&file);
        assert!(boxes.iter().any(|(kind, _)| kind == "moof"));
        assert_eq!(boxes.last().unwrap().0, "mdat");
    }

    /// Writes `target/fmp4-sample.mp4` for checking with `ffprobe`.
    #[test]
    #[ignore]
    fn writes_a_sample_file() {
        std::fs::write("target/fmp4-sample.mp4", record(3, true)).unwrap();
    }
}
