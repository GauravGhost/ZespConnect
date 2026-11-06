import { Camera, Circle, Loader2, Square } from "lucide-react";
import { useState, type RefObject } from "react";

import { readCaptureSettings } from "@/features/connected/capture";
import { Elapsed } from "@/components/app/Elapsed";
import { SCREENSHOT_LABEL, takeScreenshot } from "../capture";
import type { Recording } from "../recording";
import { MirrorIconButton } from "./MirrorIconButton";

const RECORD_LABEL = {
  folder: "Record to PC folder",
  phone: "Record to phone",
} as const;

/** Screenshot and record buttons for a mirror window's title bar. */
export function CaptureButtons({
  serial,
  canvasRef,
  live,
  recording,
}: Readonly<{
  serial: string;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  live: boolean;
  recording: Recording;
}>) {
  const [shooting, setShooting] = useState(false);
  // Read on hover so the tooltips follow a change made on the Features page.
  const [settings, setSettings] = useState(readCaptureSettings);

  const shoot = () => {
    const canvas = canvasRef.current;
    if (!canvas || shooting) return;
    setShooting(true);
    void takeScreenshot(canvas, serial).finally(() => setShooting(false));
  };

  const recordIcon = recording.busy ? Loader2 : recording.recording ? Square : Circle;

  return (
    <span className="contents" onPointerEnter={() => setSettings(readCaptureSettings())}>
      <MirrorIconButton
        label={SCREENSHOT_LABEL[settings.screenshotTarget]}
        icon={shooting ? Loader2 : Camera}
        iconClassName={shooting ? "animate-spin" : undefined}
        disabled={!live || shooting}
        onClick={shoot}
      />
      <MirrorIconButton
        label={recording.recording ? "Stop recording" : RECORD_LABEL[settings.recordingTarget]}
        icon={recordIcon}
        iconClassName={
          recording.busy
            ? "animate-spin"
            : recording.recording
              ? "text-destructive fill-current"
              : "text-destructive"
        }
        disabled={(!live && !recording.recording) || recording.busy}
        onClick={() => void recording.toggle()}
      />
      {recording.startedAt !== null ? (
        <Elapsed
          since={recording.startedAt}
          className="text-destructive px-0.5 text-[11px] font-medium"
        />
      ) : null}
    </span>
  );
}
