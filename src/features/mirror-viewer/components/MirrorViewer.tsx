import { Channel } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Loader2, MonitorX } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import { H264Decoder } from "../decoder";

type Status = "connecting" | "live" | "error";

/** The session id encoded in this window's label (`mirror-<id>`). */
function sessionId(): number | null {
  const label = getCurrentWebviewWindow().label;
  if (!label.startsWith("mirror-")) return null;
  const id = Number.parseInt(label.slice("mirror-".length), 10);
  return Number.isFinite(id) ? id : null;
}

export function MirrorViewer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const liveRef = useRef(false);
  const [status, setStatus] = useState<Status>("connecting");
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const id = sessionId();
    if (id === null) {
      setStatus("error");
      setMessage("This window is not a mirror session.");
      return;
    }

    let disposed = false;
    let unlistenExited: (() => void) | undefined;
    const canvas = canvasRef.current;
    const context = canvas?.getContext("2d") ?? null;

    const decoder = new H264Decoder({
      onFrame: (frame) => {
        if (canvas && context) {
          if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
            canvas.width = frame.displayWidth;
            canvas.height = frame.displayHeight;
          }
          context.drawImage(frame, 0, 0, frame.displayWidth, frame.displayHeight);
        }
        frame.close();

        if (!disposed && !liveRef.current) {
          liveRef.current = true;
          setStatus("live");
        }
      },
      onError: (error) => {
        if (!disposed) {
          setStatus("error");
          setMessage(error.message);
        }
      },
    });

    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (payload) => decoder.push(payload);

    ipc.mirror
      .startStream(id, channel)
      .catch(async (failure) => {
        if (disposed) return;
        setStatus("error");
        setMessage(errorMessage(failure));
        // This webview cannot render the stream — reopen as an external window.
        await ipc.mirror.fallback(id).catch(() => undefined);
      });

    events
      .mirrorSessionExited((exited) => {
        if (exited.id === id) void ipc.mirror.stopSession(id).catch(() => undefined);
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlistenExited = unlisten;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlistenExited?.();
      channel.onmessage = () => {};
      decoder.close();
    };
  }, []);

  return (
    <div className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-neutral-950">
      <canvas ref={canvasRef} className="max-h-full max-w-full object-contain" />
      {status !== "live" ? (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-neutral-950/80 text-center">
          {status === "error" ? (
            <>
              <MonitorX className="size-6 text-neutral-400" />
              <p className="max-w-sm px-6 text-sm text-neutral-300">{message}</p>
            </>
          ) : (
            <>
              <Loader2 className="size-6 animate-spin text-neutral-400" />
              <p className="text-sm text-neutral-400">Starting mirror…</p>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}
