import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ScreenshotTarget = "clipboard" | "folder" | "phone";
export type RecordingTarget = "folder" | "phone";

export interface CaptureSettings {
  screenshotTarget: ScreenshotTarget;
  recordingTarget: RecordingTarget;
  /** `null` means the backend's default folder. */
  screenshotDir: string | null;
  recordingDir: string | null;
}

interface CaptureStore extends CaptureSettings {
  update: (patch: Partial<CaptureSettings>) => void;
}

export const CAPTURE_STORAGE_KEY = "zesp-connect-capture";

const DEFAULTS: CaptureSettings = {
  screenshotTarget: "clipboard",
  recordingTarget: "folder",
  screenshotDir: null,
  recordingDir: null,
};

/** Set on the Features page, read by every mirror window. */
export const useCaptureSettings = create<CaptureStore>()(
  persist(
    (set) => ({
      ...DEFAULTS,
      update: (patch) => set(patch),
    }),
    {
      name: CAPTURE_STORAGE_KEY,
      version: 1,
      partialize: ({ screenshotTarget, recordingTarget, screenshotDir, recordingDir }) => ({
        screenshotTarget,
        recordingTarget,
        screenshotDir,
        recordingDir,
      }),
    },
  ),
);

/**
 * The settings as last saved by any window. A mirror window's in-memory store does not see
 * changes made on the Features page, so it reads storage at click time.
 */
export function readCaptureSettings(): CaptureSettings {
  try {
    const raw = localStorage.getItem(CAPTURE_STORAGE_KEY);
    const saved = raw ? (JSON.parse(raw) as { state?: Partial<CaptureSettings> }).state : null;
    return { ...DEFAULTS, ...saved };
  } catch {
    return { ...DEFAULTS };
  }
}
