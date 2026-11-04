import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";

import { useBackendEvents } from "@/hooks/useBackendEvents";
import { events } from "@/lib/events";
import type { SavedCapture } from "@/types";

/** Toast for a saved screenshot or recording, offering to show a computer file. */
export function announceSaved(
  kind: "Screenshot" | "Recording",
  saved: SavedCapture,
  notice?: string,
) {
  if (saved.onPhone) {
    toast.success(`${kind} saved on the phone`, { description: notice });
    return;
  }
  toast.success(`${kind} saved`, {
    description: notice,
    action: {
      label: "Show",
      onClick: () => void revealItemInDir(saved.location).catch(() => undefined),
    },
  });
}

/** In the main window: results of recordings whose own window is gone. */
export function useRecordingResults() {
  useBackendEvents((track) => {
    track(
      events.recordingFinished(({ payload }) => {
        if (payload.inWindow) return;
        if (payload.outcome)
          announceSaved("Recording", payload.outcome, payload.outcome.notice ?? undefined);
        else if (payload.error) toast.error(payload.error);
      }),
    );
  }, []);
}
