/** Classifies device files by extension so the browser can show a matching icon. */

export type FileKind =
  "image" | "video" | "audio" | "document" | "spreadsheet" | "archive" | "app" | "code" | "other";

const KIND_BY_EXTENSION: Record<string, FileKind> = Object.fromEntries(
  (
    [
      ["image", ["jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "bmp", "svg", "dng", "avif"]],
      ["video", ["mp4", "mkv", "mov", "webm", "3gp", "avi", "m4v", "ts"]],
      ["audio", ["mp3", "m4a", "aac", "wav", "flac", "ogg", "opus", "amr", "mid"]],
      ["document", ["pdf", "txt", "md", "doc", "docx", "rtf", "odt", "epub", "ppt", "pptx"]],
      ["spreadsheet", ["xls", "xlsx", "csv", "ods"]],
      ["archive", ["zip", "rar", "7z", "tar", "gz", "tgz", "xz", "bz2"]],
      ["app", ["apk", "apks", "xapk", "aab"]],
      ["code", ["json", "xml", "html", "js", "ts", "kt", "java", "py", "sh", "log", "yaml", "yml"]],
    ] as const
  ).flatMap(([kind, extensions]) => extensions.map((extension) => [extension, kind])),
);

export function fileKind(name: string): FileKind {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return "other";
  return KIND_BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? "other";
}
