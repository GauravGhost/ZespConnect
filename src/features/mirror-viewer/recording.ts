import { useState } from "react";
import { toast } from "sonner";

import { announceSaved } from "@/features/connected/announce";
import { readCaptureSettings } from "@/features/connected/capture";
import { useBackendEvents } from "@/hooks/useBackendEvents";
import { events } from "@/lib/events";
import { fileStamp } from "@/lib/format";
import { errorMessage, ipc } from "@/lib/ipc";

/** This window's recording: whether it runs, since when, and a start/stop toggle. */
export function useRecording(sessionId: number | null) {
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  useBackendEvents(
    (track) => {
      if (sessionId === null) return;
      void ipc.capture
        .recordingState(sessionId)
        .then(setStartedAt)
        .catch(() => undefined);
      track(
        events.recordingChanged(({ payload }) => {
          if (payload.id === sessionId) setStartedAt(payload.recording ? payload.startedAt : null);
        }),
      );
      track(
        events.recordingFinished(({ payload }) => {
          if (payload.id !== sessionId || !payload.inWindow) return;
          if (payload.outcome) {
            announceSaved("Recording", payload.outcome, payload.outcome.notice ?? undefined);
          } else if (payload.error) {
            toast.error(payload.error);
          }
        }),
      );
    },
    [sessionId],
  );

  const toggle = async () => {
    if (sessionId === null || busy) return;
    setBusy(true);
    try {
      if (startedAt !== null) {
        const outcome = await ipc.capture.stopRecording(sessionId);
        setStartedAt(null);
        announceSaved("Recording", outcome, outcome.notice ?? undefined);
      } else {
        const { recordingTarget, recordingDir } = readCaptureSettings();
        const started = await ipc.capture.startRecording(
          sessionId,
          recordingTarget,
          recordingDir,
          fileStamp(new Date()),
        );
        setStartedAt(started);
      }
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  return { recording: startedAt !== null, startedAt, busy, toggle };
}

export type Recording = ReturnType<typeof useRecording>;
