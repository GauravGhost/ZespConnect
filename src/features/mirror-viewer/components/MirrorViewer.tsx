import { Channel } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Loader2, MonitorX } from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { TouchAction } from "@/types";
import { H264Decoder } from "../decoder";

type Status = "connecting" | "live" | "error";

/**
 * How long without a decoded frame, while data is still arriving, means the
 * decoder has stalled rather than simply gone idle (the device changed nothing).
 */
const STALL_MS = 1500;
/** How often the stall watchdog runs. */
const WATCHDOG_MS = 500;

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
  const sessionRef = useRef<number | null>(null);
  const aspectRef = useRef<number | null>(null);
  const activePointerRef = useRef<number | null>(null);
  const moveFrameRef = useRef<number | null>(null);
  const pendingMoveRef = useRef<{ x: number; y: number } | null>(null);
  const sendQueueRef = useRef<Promise<void>>(Promise.resolve());
  const [status, setStatus] = useState<Status>("connecting");
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const id = sessionId();
    sessionRef.current = id;
    if (id === null) {
      setStatus("error");
      setMessage("This window is not a mirror session.");
      return;
    }

    let disposed = false;
    let unlistenExited: (() => void) | undefined;
    let lastFrameAt = performance.now();
    let lastDataAt = performance.now();
    const canvas = canvasRef.current;
    // The canvas is opaque (it only ever shows video), so skipping the alpha
    // channel lets the compositor blend it more cheaply.
    const context = canvas?.getContext("2d", { alpha: false }) ?? null;

    const decoder = new H264Decoder({
      onFrame: (frame) => {
        lastFrameAt = performance.now();
        try {
          if (canvas && context) {
            if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
              canvas.width = frame.displayWidth;
              canvas.height = frame.displayHeight;
              // Shape the window to the video's aspect ratio (and re-shape on
              // rotation). A pure resolution change keeps the same ratio, so
              // re-sizing the window would only fight the user's own resize.
              const aspect = frame.displayWidth / frame.displayHeight;
              if (aspectRef.current === null || Math.abs(aspect - aspectRef.current) > 0.001) {
                aspectRef.current = aspect;
                void ipc.mirror
                  .fitWindow(id, frame.displayWidth, frame.displayHeight)
                  .catch(() => {});
              }
            }
            context.drawImage(frame, 0, 0, frame.displayWidth, frame.displayHeight);
          }
        } finally {
          // Drawing can throw while the GPU surface is rebuilt during a resize. A
          // frame left unclosed stays in the decoder's pool and, after ~16 of them,
          // silences it for good — the "black screen with no error" symptom.
          frame.close();
        }

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
    channel.onmessage = (payload) => {
      lastDataAt = performance.now();
      decoder.push(payload);
    };

    // The decoder can go quiet without firing `error` — a hardware hiccup or GPU
    // reset while the window is being resized — leaving the canvas frozen or black
    // even though `status` still reads "live". If frames keep arriving but none
    // come out, rebuild the decoder so it resumes at the next key frame.
    const watchdog = window.setInterval(() => {
      const now = performance.now();
      if (disposed || !liveRef.current) return;
      if (now - lastFrameAt > STALL_MS && now - lastDataAt < STALL_MS) {
        lastFrameAt = now;
        decoder.reset();
      }
    }, WATCHDOG_MS);

    ipc.mirror.startStream(id, channel).catch(async (failure) => {
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
      window.clearInterval(watchdog);
      unlistenExited?.();
      channel.onmessage = () => {};
      decoder.close();
      if (moveFrameRef.current !== null) cancelAnimationFrame(moveFrameRef.current);
    };
  }, []);

  /** Queue one input message; the chain keeps down/move/up strictly ordered. */
  const send = useCallback((action: TouchAction, x: number, y: number) => {
    const id = sessionRef.current;
    const canvas = canvasRef.current;
    if (id === null || !canvas || canvas.width === 0 || canvas.height === 0) return;
    const { width, height } = canvas;
    sendQueueRef.current = sendQueueRef.current
      .then(() => ipc.mirror.touch(id, action, x, y, width, height))
      .catch(() => {});
  }, []);

  const flushMove = useCallback(() => {
    moveFrameRef.current = null;
    const point = pendingMoveRef.current;
    if (!point) return;
    pendingMoveRef.current = null;
    send("move", point.x, point.y);
  }, [send]);

  const pointFor = useCallback((event: ReactPointerEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || canvas.width === 0 || canvas.height === 0) return null;
    const rect = canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return null;

    const x = ((event.clientX - rect.left) / rect.width) * canvas.width;
    const y = ((event.clientY - rect.top) / rect.height) * canvas.height;
    return {
      x: Math.min(Math.max(Math.round(x), 0), canvas.width - 1),
      y: Math.min(Math.max(Math.round(y), 0), canvas.height - 1),
    };
  }, []);

  const onPointerDown = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (activePointerRef.current !== null) return;
    if (event.pointerType === "mouse" && event.button !== 0) return;
    const point = pointFor(event);
    if (!point) return;
    activePointerRef.current = event.pointerId;
    event.currentTarget.setPointerCapture(event.pointerId);
    send("down", point.x, point.y);
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (activePointerRef.current !== event.pointerId) return;
    const point = pointFor(event);
    if (!point) return;
    pendingMoveRef.current = point;
    if (moveFrameRef.current === null) moveFrameRef.current = requestAnimationFrame(flushMove);
  };

  const onPointerEnd = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (activePointerRef.current !== event.pointerId) return;
    activePointerRef.current = null;
    if (moveFrameRef.current !== null) {
      cancelAnimationFrame(moveFrameRef.current);
      moveFrameRef.current = null;
    }
    pendingMoveRef.current = null;
    const point = pointFor(event);
    if (point) send("up", point.x, point.y);
  };

  return (
    <div className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-neutral-950">
      <canvas
        ref={canvasRef}
        className="max-h-full max-w-full touch-none"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerEnd}
        onPointerCancel={onPointerEnd}
      />
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
