import { writeImage } from "@tauri-apps/plugin-clipboard-manager";
import { toast } from "sonner";

import { announceSaved } from "@/features/connected/announce";
import { readCaptureSettings, type ScreenshotTarget } from "@/features/connected/capture";
import { errorMessage, ipc } from "@/lib/ipc";
import { fileStamp } from "@/lib/format";

export const SCREENSHOT_LABEL: Record<ScreenshotTarget, string> = {
  clipboard: "Screenshot to clipboard",
  folder: "Screenshot to PC folder",
  phone: "Screenshot to phone",
};

function canvasPng(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("Nothing on screen to capture yet"));
        return;
      }
      blob.arrayBuffer().then((buffer) => resolve(new Uint8Array(buffer)), reject);
    }, "image/png");
  });
}

/** Capture the frame on `canvas` and send it where the Features page says. */
export async function takeScreenshot(canvas: HTMLCanvasElement, serial: string): Promise<void> {
  const { screenshotTarget, screenshotDir } = readCaptureSettings();
  try {
    const png = await canvasPng(canvas);
    if (screenshotTarget === "clipboard") {
      await writeImage(png);
      toast.success("Screenshot copied");
      return;
    }
    const saved = await ipc.capture.saveScreenshot(
      serial,
      png,
      screenshotTarget,
      screenshotDir,
      fileStamp(new Date()),
    );
    announceSaved("Screenshot", saved);
  } catch (error) {
    toast.error(errorMessage(error));
  }
}
