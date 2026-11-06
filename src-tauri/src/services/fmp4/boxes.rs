use super::{AudioConfig, VideoConfig};

pub const VIDEO_TRACK: u32 = 1;
pub const AUDIO_TRACK: u32 = 2;
pub const VIDEO_TIMESCALE: u32 = 90_000;
pub const MOVIE_TIMESCALE: u32 = 1000;
/// `sample_depends_on = 2`: a key frame.
pub const SYNC_SAMPLE: u32 = 0x0200_0000;
/// `sample_depends_on = 1` and `sample_is_non_sync_sample`.
pub const NON_SYNC_SAMPLE: u32 = 0x0101_0000;
const UNITY_MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];
/// "und" packed as three 5-bit letters.
const LANGUAGE_UNDETERMINED: u16 = 0x55C4;

pub fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

fn full(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(body.len() + 4);
    content.extend_from_slice(&((u32::from(version) << 24) | flags).to_be_bytes());
    content.extend_from_slice(body);
    bx(kind, &content)
}

#[derive(Default)]
struct Bytes(Vec<u8>);

impl Bytes {
    fn u8(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }
    fn u16(&mut self, value: u16) -> &mut Self {
        self.0.extend_from_slice(&value.to_be_bytes());
        self
    }
    fn u32(&mut self, value: u32) -> &mut Self {
        self.0.extend_from_slice(&value.to_be_bytes());
        self
    }
    fn zeros(&mut self, count: usize) -> &mut Self {
        self.0.resize(self.0.len() + count, 0);
        self
    }
    fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.extend_from_slice(bytes);
        self
    }
    fn matrix(&mut self) -> &mut Self {
        for value in UNITY_MATRIX {
            self.u32(value);
        }
        self
    }
    fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

pub fn ftyp() -> Vec<u8> {
    bx(b"ftyp", b"iso6\0\0\0\0iso6isomavc1mp41")
}

/// `duration_ms` is the whole recording's length; 0 while it is still being written.
pub fn moov(video: &VideoConfig, audio: Option<&AudioConfig>, duration_ms: u32) -> Vec<u8> {
    let next_track = if audio.is_some() {
        AUDIO_TRACK
    } else {
        VIDEO_TRACK
    } + 1;
    let mvhd = full(
        b"mvhd",
        0,
        0,
        &Bytes::default()
            .zeros(8)
            .u32(MOVIE_TIMESCALE)
            .u32(duration_ms)
            .u32(0x0001_0000)
            .u16(0x0100)
            .zeros(10)
            .matrix()
            .zeros(24)
            .u32(next_track)
            .take(),
    );

    let mut body = mvhd;
    body.extend(video_trak(video));
    // Players read a fragmented file's length from `mehd`.
    let mut mvex = full(b"mehd", 0, 0, &duration_ms.to_be_bytes());
    mvex.extend(trex(VIDEO_TRACK));
    if let Some(audio) = audio {
        body.extend(audio_trak(audio));
        mvex.extend(trex(AUDIO_TRACK));
    }
    body.extend(bx(b"mvex", &mvex));
    bx(b"moov", &body)
}

fn trex(track: u32) -> Vec<u8> {
    full(
        b"trex",
        0,
        0,
        &Bytes::default()
            .u32(track)
            .u32(1)
            .u32(0)
            .u32(0)
            .u32(0)
            .take(),
    )
}

fn tkhd(track: u32, volume: u16, width: u32, height: u32) -> Vec<u8> {
    // Flags: enabled and in the movie.
    full(
        b"tkhd",
        0,
        3,
        &Bytes::default()
            .zeros(8)
            .u32(track)
            .zeros(4)
            .u32(0)
            .zeros(8)
            .u16(0)
            .u16(0)
            .u16(volume)
            .zeros(2)
            .matrix()
            .u32(width << 16)
            .u32(height << 16)
            .take(),
    )
}

fn mdia(
    timescale: u32,
    handler: &[u8; 4],
    name: &str,
    media_header: Vec<u8>,
    stsd: Vec<u8>,
) -> Vec<u8> {
    let mdhd = full(
        b"mdhd",
        0,
        0,
        &Bytes::default()
            .zeros(8)
            .u32(timescale)
            .u32(0)
            .u16(LANGUAGE_UNDETERMINED)
            .u16(0)
            .take(),
    );
    let hdlr = full(
        b"hdlr",
        0,
        0,
        &Bytes::default()
            .u32(0)
            .raw(handler)
            .zeros(12)
            .raw(name.as_bytes())
            .u8(0)
            .take(),
    );
    let url = full(b"url ", 0, 1, &[]);
    let dinf = bx(
        b"dinf",
        &full(b"dref", 0, 0, &[&1u32.to_be_bytes()[..], &url].concat()),
    );
    let empty_table = |kind: &[u8; 4]| full(kind, 0, 0, &0u32.to_be_bytes());
    let stbl = bx(
        b"stbl",
        &[
            stsd,
            empty_table(b"stts"),
            empty_table(b"stsc"),
            full(b"stsz", 0, 0, &[0u8; 8]),
            empty_table(b"stco"),
        ]
        .concat(),
    );
    let minf = bx(b"minf", &[media_header, dinf, stbl].concat());
    bx(b"mdia", &[mdhd, hdlr, minf].concat())
}

fn video_trak(video: &VideoConfig) -> Vec<u8> {
    let avcc = bx(
        b"avcC",
        &Bytes::default()
            .u8(1)
            .raw(&video.sps[1..4])
            // 4-byte NAL lengths, then one SPS and one PPS.
            .u8(0xFF)
            .u8(0xE1)
            .u16(video.sps.len() as u16)
            .raw(&video.sps)
            .u8(1)
            .u16(video.pps.len() as u16)
            .raw(&video.pps)
            .take(),
    );
    let avc1 = bx(
        b"avc1",
        &Bytes::default()
            .zeros(6)
            .u16(1)
            .zeros(16)
            .u16(video.width as u16)
            .u16(video.height as u16)
            .u32(0x0048_0000)
            .u32(0x0048_0000)
            .u32(0)
            .u16(1)
            .zeros(32)
            .u16(0x0018)
            .u16(0xFFFF)
            .raw(&avcc)
            .take(),
    );
    let stsd = full(b"stsd", 0, 0, &[&1u32.to_be_bytes()[..], &avc1].concat());
    let vmhd = full(b"vmhd", 0, 1, &[0u8; 8]);
    bx(
        b"trak",
        &[
            tkhd(VIDEO_TRACK, 0, video.width, video.height),
            mdia(VIDEO_TIMESCALE, b"vide", "Zesp video", vmhd, stsd),
        ]
        .concat(),
    )
}

/// An MPEG-4 descriptor with a one-byte length (every descriptor here is under 128 bytes).
fn descriptor(tag: u8, body: &[u8]) -> Vec<u8> {
    [&[tag, body.len() as u8][..], body].concat()
}

fn audio_trak(audio: &AudioConfig) -> Vec<u8> {
    let decoder_specific = descriptor(5, &audio.asc);
    let decoder_config = descriptor(
        4,
        &Bytes::default()
            // MPEG-4 audio, audio stream.
            .u8(0x40)
            .u8(0x15)
            .zeros(3)
            .u32(0)
            .u32(0)
            .raw(&decoder_specific)
            .take(),
    );
    let es = descriptor(
        3,
        &Bytes::default()
            .u16(AUDIO_TRACK as u16)
            .u8(0)
            .raw(&decoder_config)
            .raw(&descriptor(6, &[2]))
            .take(),
    );
    let esds = full(b"esds", 0, 0, &es);
    let mp4a = bx(
        b"mp4a",
        &Bytes::default()
            .zeros(6)
            .u16(1)
            .zeros(8)
            .u16(audio.channels)
            .u16(16)
            .zeros(4)
            .u32(audio.sample_rate << 16)
            .raw(&esds)
            .take(),
    );
    let stsd = full(b"stsd", 0, 0, &[&1u32.to_be_bytes()[..], &mp4a].concat());
    let smhd = full(b"smhd", 0, 0, &[0u8; 4]);
    bx(
        b"trak",
        &[
            tkhd(AUDIO_TRACK, 0x0100, 0, 0),
            mdia(audio.sample_rate, b"soun", "Zesp sound", smhd, stsd),
        ]
        .concat(),
    )
}

pub struct Sample {
    pub duration: u32,
    pub flags: u32,
    pub data: Vec<u8>,
}

/// One track's fragment: `moof` then `mdat` holding `samples` in order.
pub fn fragment(sequence: u32, track: u32, decode_time: u64, samples: &[Sample]) -> Vec<u8> {
    let moof = |data_offset: u32| {
        let mfhd = full(b"mfhd", 0, 0, &sequence.to_be_bytes());
        // default-base-is-moof
        let tfhd = full(b"tfhd", 0, 0x02_0000, &track.to_be_bytes());
        let tfdt = full(b"tfdt", 1, 0, &decode_time.to_be_bytes());
        let mut trun = Bytes::default();
        trun.u32(samples.len() as u32).u32(data_offset);
        for sample in samples {
            trun.u32(sample.duration)
                .u32(sample.data.len() as u32)
                .u32(sample.flags);
        }
        // data-offset, sample duration, size and flags present.
        let trun = full(b"trun", 0, 0x0701, &trun.take());
        let traf = bx(b"traf", &[tfhd, tfdt, trun].concat());
        bx(b"moof", &[mfhd, traf].concat())
    };
    let header_len = moof(0).len() as u32;
    let mut out = moof(header_len + 8);
    let payload: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.data.iter().copied())
        .collect();
    out.extend(bx(b"mdat", &payload));
    out
}
