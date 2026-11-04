//! Reads device files in byte ranges over adb so the webview can stream them without a local
//! copy. Requests arrive as `devicefile://localhost/<url-encoded serial + device path>`.
//! Range planning, URL and MIME handling are pure and unit-tested.

use std::time::Duration;

use crate::error::{AppError, Result};
use crate::models::ToolStatus;
use crate::services::adb;
use crate::util::validation::{shell_quote, validate_remote_path, validate_serial};

/// The URI scheme the webview loads device files from.
pub const SCHEME: &str = "devicefile";

/// Largest slice sent for one range request; players ask again for the rest.
pub const MAX_CHUNK: u64 = 4 * 1024 * 1024;

/// Largest file sent whole to a request without a `Range` header (images, PDFs, text).
pub const MAX_WHOLE_FILE: u64 = 64 * 1024 * 1024;

/// `dd` block size; reads start on a block boundary and the leading bytes are dropped.
const BLOCK: u64 = 64 * 1024;

const STAT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// An inclusive byte range within a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub fn whole(size: u64) -> Self {
        Self {
            start: 0,
            end: size.saturating_sub(1),
        }
    }

    pub fn byte_count(&self) -> u64 {
        self.end - self.start + 1
    }
}

/// The device a request targets and the file on it.
#[derive(Debug, PartialEq, Eq)]
pub struct StreamTarget {
    pub serial: String,
    pub remote: String,
}

/// Split a request path (`/<encoded "serial/sdcard/...">`) into its device serial and file path.
pub fn parse_target(uri_path: &str) -> Result<StreamTarget> {
    let decoded = percent_decode(uri_path.trim_start_matches('/'))
        .ok_or_else(|| AppError::Invalid("Malformed file address".into()))?;
    let slash = decoded
        .find('/')
        .ok_or_else(|| AppError::Invalid("Missing file path".into()))?;
    let (serial, remote) = decoded.split_at(slash);
    validate_serial(serial)?;
    validate_remote_path(remote)?;
    Ok(StreamTarget {
        serial: serial.to_string(),
        remote: remote.to_string(),
    })
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// The range to send for a `Range` header against a `size`-byte file, capped at [`MAX_CHUNK`].
/// Supports `bytes=a-b`, `bytes=a-` and `bytes=-n`; only the first of several ranges is served.
/// `None` when the range cannot be satisfied.
pub fn range_for(header: &str, size: u64) -> Option<ByteRange> {
    let spec = header
        .trim()
        .strip_prefix("bytes=")?
        .split(',')
        .next()?
        .trim();
    let (first, last) = spec.split_once('-')?;
    if size == 0 {
        return None;
    }

    let (start, end) = if first.is_empty() {
        let suffix: u64 = last.parse().ok()?;
        (size.saturating_sub(suffix), size - 1)
    } else {
        let start: u64 = first.parse().ok()?;
        let end = if last.is_empty() {
            size - 1
        } else {
            last.parse::<u64>().ok()?.min(size - 1)
        };
        (start, end)
    };
    if start > end || start >= size {
        return None;
    }
    Some(ByteRange {
        start,
        end: end.min(start + MAX_CHUNK - 1),
    })
}

/// `dd` reading the blocks that cover `range`, plus how many leading bytes to drop from its output.
fn read_command(remote: &str, range: ByteRange) -> (String, usize) {
    let skip = range.start / BLOCK;
    let offset = range.start % BLOCK;
    let count = (offset + range.byte_count()).div_ceil(BLOCK);
    let command = format!(
        "dd if={} bs={BLOCK} skip={skip} count={count} 2>/dev/null",
        shell_quote(remote)
    );
    (command, offset as usize)
}

/// The `Content-Type` for a device file, by extension.
pub fn mime_type(remote: &str) -> &'static str {
    let extension = remote
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "heic" | "heif" => "image/heic",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "3gp" => "video/3gpp",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" | "opus" => "audio/ogg",
        "pdf" => "application/pdf",
        "txt" | "md" | "log" | "csv" | "json" | "xml" | "yaml" | "yml" | "sh" | "kt" | "java"
        | "py" | "js" | "ts" | "html" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub async fn file_size(tools: &ToolStatus, serial: &str, remote: &str) -> Result<u64> {
    let adb_path = adb::adb_path(tools)?;
    let command = format!("stat -L -c %s {}", shell_quote(remote));
    let output = adb::shell(&adb_path, serial, &command, STAT_TIMEOUT).await?;
    output
        .stdout
        .trim()
        .parse()
        .map_err(|_| AppError::CommandFailed(format!("Could not read {remote}: {}", output.text())))
}

pub async fn read_range(
    tools: &ToolStatus,
    serial: &str,
    remote: &str,
    range: ByteRange,
) -> Result<Vec<u8>> {
    let adb_path = adb::adb_path(tools)?;
    let (command, offset) = read_command(remote, range);
    let mut bytes = adb::exec_out(&adb_path, serial, &command, READ_TIMEOUT).await?;
    bytes.drain(..offset.min(bytes.len()));
    bytes.truncate(range.byte_count() as usize);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_serial_and_path_from_the_request() {
        let target = parse_target("/192.168.1.5%3A5555%2Fsdcard%2FDCIM%2FMy%20Photo.jpg").unwrap();
        assert_eq!(
            target,
            StreamTarget {
                serial: "192.168.1.5:5555".into(),
                remote: "/sdcard/DCIM/My Photo.jpg".into(),
            }
        );
        assert!(parse_target("/only-a-serial").is_err());
        assert!(parse_target("/abc%2").is_err());
    }

    #[test]
    fn resolves_open_ended_and_suffix_ranges() {
        assert_eq!(
            range_for("bytes=0-1", 100),
            Some(ByteRange { start: 0, end: 1 })
        );
        assert_eq!(
            range_for("bytes=10-", 100),
            Some(ByteRange { start: 10, end: 99 })
        );
        assert_eq!(
            range_for("bytes=-20", 100),
            Some(ByteRange { start: 80, end: 99 })
        );
        assert_eq!(
            range_for("bytes=90-500", 100),
            Some(ByteRange { start: 90, end: 99 })
        );
    }

    #[test]
    fn caps_ranges_at_one_chunk() {
        let range = range_for("bytes=0-", 10 * MAX_CHUNK).unwrap();
        assert_eq!(range.byte_count(), MAX_CHUNK);
    }

    #[test]
    fn rejects_unsatisfiable_ranges() {
        assert_eq!(range_for("bytes=100-", 100), None);
        assert_eq!(range_for("bytes=5-2", 100), None);
        assert_eq!(range_for("bytes=0-", 0), None);
        assert_eq!(range_for("items=0-1", 100), None);
    }

    #[test]
    fn reads_whole_blocks_and_drops_the_leading_offset() {
        let (command, offset) = read_command(
            "/sdcard/a b.mp4",
            ByteRange {
                start: BLOCK + 10,
                end: BLOCK + 19,
            },
        );
        assert_eq!(
            command,
            format!("dd if='/sdcard/a b.mp4' bs={BLOCK} skip=1 count=1 2>/dev/null")
        );
        assert_eq!(offset, 10);

        let (command, _) = read_command(
            "/f",
            ByteRange {
                start: BLOCK - 1,
                end: BLOCK,
            },
        );
        assert!(command.contains("skip=0 count=2"));
    }

    #[test]
    fn picks_content_types_by_extension() {
        assert_eq!(mime_type("/sdcard/DCIM/IMG.JPG"), "image/jpeg");
        assert_eq!(mime_type("/sdcard/Movies/clip.mov"), "video/quicktime");
        assert_eq!(mime_type("/sdcard/notes"), "application/octet-stream");
    }
}
