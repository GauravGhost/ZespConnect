/**
 * H.264 Annex-B decoder built on WebCodecs, fed by the framing the Rust side
 * sends over the IPC channel: `[flags][8-byte PTS][payload]`, where `flags` bit 0
 * marks a key frame and bit 1 a codec-config (SPS/PPS) packet.
 */

const FLAG_KEY = 0x1;
const FLAG_CONFIG = 0x2;
const HEADER_LEN = 9;

export interface DecoderHandlers {
  onFrame: (frame: VideoFrame) => void;
  onError: (error: Error) => void;
}

export class H264Decoder {
  private decoder: VideoDecoder | null = null;
  private config: Uint8Array | null = null;
  private readonly handlers: DecoderHandlers;

  constructor(handlers: DecoderHandlers) {
    this.handlers = handlers;
  }

  push(message: ArrayBuffer): void {
    if (message.byteLength <= HEADER_LEN) return;

    const view = new DataView(message);
    const flags = view.getUint8(0);
    const payload = new Uint8Array(message, HEADER_LEN);

    if (flags & FLAG_CONFIG) {
      this.config = payload.slice();
      return;
    }

    const key = (flags & FLAG_KEY) !== 0;
    if (!this.ensureDecoder(this.config ?? payload)) return;

    const data = key && this.config ? concat(this.config, payload) : payload;
    try {
      this.decoder?.decode(
        new EncodedVideoChunk({
          type: key ? "key" : "delta",
          timestamp: Number(view.getBigUint64(1)),
          data,
        }),
      );
    } catch (error) {
      this.handlers.onError(toError(error));
    }
  }

  close(): void {
    try {
      this.decoder?.close();
    } catch {
      // Already closed.
    }
    this.decoder = null;
  }

  private ensureDecoder(bytes: Uint8Array): boolean {
    if (this.decoder) return true;

    const codec = codecString(bytes);
    if (!codec) return false;

    try {
      this.decoder = new VideoDecoder({
        output: (frame) => this.handlers.onFrame(frame),
        error: (error) => this.handlers.onError(error),
      });
      this.decoder.configure({ codec, optimizeForLatency: true });
      return true;
    } catch (error) {
      this.handlers.onError(toError(error));
      this.decoder = null;
      return false;
    }
  }
}

/** Build the `avc1.PPCCLL` codec string from the SPS profile/constraint/level bytes. */
function codecString(bytes: Uint8Array): string | null {
  for (let index = 0; index + 4 < bytes.length; index += 1) {
    if (bytes[index] !== 0 || bytes[index + 1] !== 0) continue;

    let nalStart: number;
    if (bytes[index + 2] === 1) {
      nalStart = index + 3;
    } else if (bytes[index + 2] === 0 && bytes[index + 3] === 1) {
      nalStart = index + 4;
    } else {
      continue;
    }

    if (nalStart + 3 >= bytes.length) continue;
    if ((bytes[nalStart] & 0x1f) !== 7) continue;

    const profile = bytes[nalStart + 1];
    const constraints = bytes[nalStart + 2];
    const level = bytes[nalStart + 3];
    return `avc1.${hex(profile)}${hex(constraints)}${hex(level)}`;
  }
  return null;
}

function hex(value: number): string {
  return value.toString(16).padStart(2, "0");
}

function concat(left: Uint8Array, right: Uint8Array): Uint8Array {
  const merged = new Uint8Array(left.length + right.length);
  merged.set(left, 0);
  merged.set(right, left.length);
  return merged;
}

function toError(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value));
}
