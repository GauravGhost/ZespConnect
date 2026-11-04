import { Camera, Loader2 } from "lucide-react";
import { useState, type RefObject } from "react";

import { readCaptureSettings } from "@/features/connected/capture";
import { SCREENSHOT_LABEL, takeScreenshot } from "../capture";
import { MirrorIconButton } from "./MirrorIconButton";

/** Screenshot and record buttons for a mirror window's title bar. */
export function CaptureButtons({
  serial,
  canvasRef,
  live,
}: Readonly<{
  serial: string;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  live: boolean;
}>) {
  const [shooting, setShooting] = useState(false);
  // Read on hover so the tooltip follows a change made on the Features page.
  const [label, setLabel] = useState(SCREENSHOT_LABEL[readCaptureSettings().screenshotTarget]);

  const shoot = () => {
    const canvas = canvasRef.current;
    if (!canvas || shooting) return;
    setShooting(true);
    void takeScreenshot(canvas, serial).finally(() => setShooting(false));
  };

  return (
    <span
      className="contents"
      onPointerEnter={() => setLabel(SCREENSHOT_LABEL[readCaptureSettings().screenshotTarget])}
    >
      <MirrorIconButton
        label={label}
        icon={shooting ? Loader2 : Camera}
        iconClassName={shooting ? "animate-spin" : undefined}
        disabled={!live || shooting}
        onClick={shoot}
      />
    </span>
  );
}
