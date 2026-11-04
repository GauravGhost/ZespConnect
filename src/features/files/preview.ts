import { convertFileSrc } from "@tauri-apps/api/core";

/** How the in-app viewer shows a file; files without one open in the computer's default app. */
export type PreviewKind = "image" | "video" | "audio" | "pdf" | "text";

/** Text files above this size open in the default app instead of the viewer. */
export const TEXT_PREVIEW_LIMIT = 1024 * 1024;

/** Must match `services::file_stream::SCHEME` in the backend. */
const DEVICE_FILE_SCHEME = "devicefile";

const PREVIEW_BY_EXTENSION: Record<string, PreviewKind> = Object.fromEntries(
  (
    [
      ["image", ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "avif", "heic", "heif"]],
      ["video", ["mp4", "m4v", "mov", "webm", "3gp"]],
      ["audio", ["mp3", "m4a", "aac", "wav", "flac", "ogg", "opus"]],
      ["pdf", ["pdf"]],
      [
        "text",
        [
          "txt",
          "md",
          "log",
          "csv",
          "json",
          "xml",
          "yaml",
          "yml",
          "sh",
          "kt",
          "java",
          "py",
          "js",
          "ts",
          "html",
        ],
      ],
    ] as const
  ).flatMap(([kind, extensions]) => extensions.map((extension) => [extension, kind])),
);

export function previewKind(name: string, size: number): PreviewKind | null {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return null;
  const kind = PREVIEW_BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? null;
  return kind === "text" && size > TEXT_PREVIEW_LIMIT ? null : kind;
}

/** A URL the webview can load a device file from, streamed over adb by the backend. */
export function deviceFileUrl(serial: string, remote: string): string {
  return convertFileSrc(`${serial}${remote}`, DEVICE_FILE_SCHEME);
}
