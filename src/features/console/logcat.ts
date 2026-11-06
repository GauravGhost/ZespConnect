import type { LogLine } from "@/types";

export type LogLevel = "V" | "D" | "I" | "W" | "E" | "F";

export const LOG_LEVELS: { id: LogLevel; label: string }[] = [
  { id: "V", label: "Verbose" },
  { id: "D", label: "Debug" },
  { id: "I", label: "Info" },
  { id: "W", label: "Warning" },
  { id: "E", label: "Error" },
  { id: "F", label: "Fatal" },
];

export interface ParsedLine {
  text: string;
  /** adb's own errors arrive on stderr and have no level or tag. */
  stderr: boolean;
  level: LogLevel | null;
  tag: string | null;
}

export interface LogQuery {
  tag: string;
  search: string;
}

/** `threadtime` format: date, time, pid, tid, level, then `tag: message`. */
const THREADTIME = /^\S+\s+\S+\s+\d+\s+\d+\s+([VDIWEF])\s+(.*?)\s*:\s/;

export function parseLogLine(line: LogLine): ParsedLine {
  const stderr = line.stream === "stderr";
  const match = stderr ? null : THREADTIME.exec(line.text);
  return {
    text: line.text,
    stderr,
    level: (match?.[1] as LogLevel | undefined) ?? null,
    tag: match?.[2] || null,
  };
}

/** Case-insensitive tag and text match; empty fields match everything. */
export function matchesQuery(line: ParsedLine, query: LogQuery): boolean {
  const tag = query.tag.trim().toLowerCase();
  if (tag && !line.tag?.toLowerCase().includes(tag)) return false;
  const search = query.search.trim().toLowerCase();
  return !search || line.text.toLowerCase().includes(search);
}
