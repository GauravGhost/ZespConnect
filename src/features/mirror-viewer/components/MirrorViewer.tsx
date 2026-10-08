import { Channel } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Loader2, Lock, MonitorX } from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent,
} from "react";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { TouchAction } from "@/types";
import { H264Decoder } from "../decoder";
import {
  androidButton,
  androidButtons,
  androidKeycode,
  androidMetaState,
  isTextKey,
  wheelNotches,
} from "../keymap";
import { DesktopTitleBar } from "./DesktopTitleBar";

type Status = "connecting" | "live" | "error";

/**
 * How long without a decoded frame, while data is still arriving, means the
 * decoder has stalled rather than simply gone idle (the device changed nothing).
 */
const STALL_MS = 1500;
/** How often the stall watchdog runs. */
const WATCHDOG_MS = 500;
/** The least time between two key-frame requests to the device. */
const KEYFRAME_REQUEST_MS = 1000;

/** The session id encoded in this window's label (`mirror-<id>`). */
function sessionId(): number | null {
  const label = getCurrentWebviewWindow().label;
  if (!label.startsWith("mirror-")) return null;
  const id = Number.parseInt(label.slice("mirror-".length), 10);
  return Number.isFinite(id) ? id : null;
}

/** Desktop windows are opened at `index.html?desktop` (see `open_mirror_window`). */
const DESKTOP_WINDOW = new URLSearchParams(window.location.search).has("desktop");

/** Mouse buttons sent with a desktop pointer event (Android `BUTTON_*` masks). */
interface MouseButtons {
  button: number;
  buttons: number;
}

export function MirrorViewer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const liveRef = useRef(false);
  const sessionRef = useRef<number | null>(null);
  const aspectRef = useRef<number | null>(null);
  const activePointerRef = useRef<number | null>(null);
  const moveFrameRef = useRef<number | null>(null);
  const pendingMoveRef = useRef<{ x: number; y: number; hover: boolean; buttons: number } | null>(
    null,
  );
  const sendQueueRef = useRef<Promise<void>>(Promise.resolve());
  const desktopRef = useRef(DESKTOP_WINDOW);
  const pressedButtonRef = useRef(0);
  const wheelRef = useRef<{ x: number; y: number; dx: number; dy: number } | null>(null);
  const wheelFrameRef = useRef<number | null>(null);
  const [status, setStatus] = useState<Status>("connecting");
  const [message, setMessage] = useState<string | null>(null);
  // A desktop session fills its window like a monitor and takes mouse and keyboard.
  const [desktop, setDesktop] = useState(DESKTOP_WINDOW);
  // Android hides the desktop while the phone is locked (only the wallpaper draws).
  const [locked, setLocked] = useState(false);
  // Desktop mode turns the phone's own screen off; the side toggle turns it back
  // on to use the phone alongside the desktop.
  const [phoneScreen, setPhoneScreen] = useState(false);

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
    let unlistenLocked: (() => void) | undefined;
    let lastFrameAt = performance.now();
    let lastDataAt = performance.now();
    let lastKeyframeRequestAt = -Infinity;
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
      onNeedKeyframe: () => {
        // Throttled: one fresh key frame resyncs; repeated resets would only
        // restart the encoder over and over.
        const now = performance.now();
        if (now - lastKeyframeRequestAt < KEYFRAME_REQUEST_MS) return;
        lastKeyframeRequestAt = now;
        void ipc.mirror.resetVideo(id).catch(() => undefined);
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

    ipc.mirror
      .startStream(id, channel)
      .then((session) => {
        if (disposed) return;
        desktopRef.current = session.kind === "desktop";
        setDesktop(desktopRef.current);
        setPhoneScreen(session.phoneScreen);
      })
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

    events
      .mirrorSessionLocked(({ payload }) => {
        if (payload.id !== id || disposed) return;
        setLocked(payload.locked);
        setPhoneScreen(payload.phoneScreen);
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlistenLocked = unlisten;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      window.clearInterval(watchdog);
      unlistenExited?.();
      unlistenLocked?.();
      channel.onmessage = () => {};
      decoder.close();
      if (moveFrameRef.current !== null) cancelAnimationFrame(moveFrameRef.current);
      if (wheelFrameRef.current !== null) cancelAnimationFrame(wheelFrameRef.current);
    };
  }, []);

  /**
   * Queue one input message. Every kind of input (pointer, wheel, keys) goes
   * through this one chain so the device sees them strictly in order.
   */
  const enqueue = useCallback((task: (id: number) => Promise<void>) => {
    const id = sessionRef.current;
    if (id === null) return;
    sendQueueRef.current = sendQueueRef.current.then(() => task(id)).catch(() => {});
  }, []);

  /** Queue a pointer action measured against the current video frame. */
  const send = useCallback(
    (action: TouchAction, x: number, y: number, mouse?: MouseButtons) => {
      const canvas = canvasRef.current;
      if (!canvas || canvas.width === 0 || canvas.height === 0) return;
      const { width, height } = canvas;
      enqueue((id) => ipc.mirror.touch(id, action, x, y, width, height, mouse));
    },
    [enqueue],
  );

  const flushMove = useCallback(() => {
    moveFrameRef.current = null;
    const point = pendingMoveRef.current;
    if (!point) return;
    pendingMoveRef.current = null;
    if (!desktopRef.current) {
      send("move", point.x, point.y);
    } else if (point.hover) {
      send("hover", point.x, point.y, { button: 0, buttons: 0 });
    } else {
      send("move", point.x, point.y, { button: 0, buttons: point.buttons });
    }
  }, [send]);

  /** Coalesce wheel travel per frame, then scroll at the latest pointer spot. */
  const flushWheel = useCallback(() => {
    wheelFrameRef.current = null;
    const wheel = wheelRef.current;
    const canvas = canvasRef.current;
    wheelRef.current = null;
    if (!wheel || !canvas || canvas.width === 0 || canvas.height === 0) return;
    const { width, height } = canvas;
    // Browser deltas grow rightward/downward; Android scrolls right/up when positive.
    const hscroll = wheel.dx;
    const vscroll = -wheel.dy;
    enqueue((id) => ipc.mirror.scroll(id, wheel.x, wheel.y, width, height, hscroll, vscroll));
  }, [enqueue]);

  // Desktop keyboard: printable characters go as text (respecting the Mac's
  // layout), everything else — Enter, arrows, shortcuts — as Android keycodes.
  useEffect(() => {
    if (!desktop) return;
    const held = new Map<string, number>();
    const typed = new Set<string>();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.isComposing) return;
      if (isTextKey(event)) {
        event.preventDefault();
        typed.add(event.code);
        const text = event.key;
        enqueue((id) => ipc.mirror.text(id, text));
        return;
      }
      const keycode = androidKeycode(event);
      if (keycode === undefined) return;
      event.preventDefault();
      held.set(event.code, keycode);
      const meta = androidMetaState(event);
      const repeat = event.repeat ? 1 : 0;
      enqueue((id) => ipc.mirror.key(id, "down", keycode, repeat, meta));
    };

    const onKeyUp = (event: KeyboardEvent) => {
      if (typed.delete(event.code)) return;
      const keycode = held.get(event.code);
      if (keycode === undefined) return;
      event.preventDefault();
      held.delete(event.code);
      const meta = androidMetaState(event);
      enqueue((id) => ipc.mirror.key(id, "up", keycode, 0, meta));
    };

    // Losing focus mid-press would leave keys (often modifiers) stuck down.
    const releaseAll = () => {
      for (const keycode of held.values()) {
        enqueue((id) => ipc.mirror.key(id, "up", keycode, 0, 0));
      }
      held.clear();
      typed.clear();
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", releaseAll);
    return () => {
      releaseAll();
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", releaseAll);
    };
  }, [desktop, enqueue]);

  const pointFor = useCallback((event: { clientX: number; clientY: number }) => {
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

  const togglePhoneScreen = () => {
    const id = sessionRef.current;
    if (id === null) return;
    const on = !phoneScreen;
    setPhoneScreen(on);
    ipc.mirror.phoneScreen(id, on).catch(() => setPhoneScreen(!on));
  };

  /** Desktop windows forward real mouse buttons; phone windows only a left click. */
  const desktopMouse = (event: ReactPointerEvent<HTMLCanvasElement>) =>
    desktopRef.current && event.pointerType === "mouse";

  const onPointerDown = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (activePointerRef.current !== null) return;
    const mouse = desktopMouse(event);
    if (event.pointerType === "mouse" && !mouse && event.button !== 0) return;
    const point = pointFor(event);
    if (!point) return;
    activePointerRef.current = event.pointerId;
    event.currentTarget.setPointerCapture(event.pointerId);
    if (mouse) {
      const button = androidButton(event.button);
      pressedButtonRef.current = button;
      send("down", point.x, point.y, { button, buttons: androidButtons(event.buttons) || button });
    } else {
      send("down", point.x, point.y);
    }
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    // With no button held, a desktop mouse hovers (taskbar, window buttons, links).
    const hover = activePointerRef.current === null && desktopMouse(event);
    if (!hover && activePointerRef.current !== event.pointerId) return;
    const point = pointFor(event);
    if (!point) return;
    pendingMoveRef.current = { ...point, hover, buttons: androidButtons(event.buttons) };
    if (moveFrameRef.current === null) moveFrameRef.current = requestAnimationFrame(flushMove);
  };

  const onWheel = (event: ReactWheelEvent<HTMLCanvasElement>) => {
    if (!desktopRef.current) return;
    const point = pointFor(event);
    if (!point) return;
    const [dx, dy] = wheelNotches(event.nativeEvent);
    const wheel = wheelRef.current;
    wheelRef.current = { ...point, dx: (wheel?.dx ?? 0) + dx, dy: (wheel?.dy ?? 0) + dy };
    if (wheelFrameRef.current === null) wheelFrameRef.current = requestAnimationFrame(flushWheel);
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
    if (!point) return;
    if (desktopMouse(event)) {
      const button = pressedButtonRef.current;
      pressedButtonRef.current = 0;
      send("up", point.x, point.y, { button, buttons: 0 });
    } else {
      send("up", point.x, point.y);
    }
  };

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-neutral-950">
      {desktop ? (
        <DesktopTitleBar phoneScreen={phoneScreen} onTogglePhoneScreen={togglePhoneScreen} />
      ) : null}
      {/* A phone window keeps a 14px bottom gap and rounded corners, like a phone;
          a desktop fills the area under its title bar like a monitor. The window
          is sized to include either (`mirror_window::vertical_inset`). */}
      <div
        className={cn(
          "relative flex min-h-0 flex-1 items-center justify-center",
          !desktop && "pb-3.5",
        )}
      >
        <canvas
          ref={canvasRef}
          className={cn("max-h-full max-w-full touch-none", !desktop && "rounded-[12px]")}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerEnd}
          onPointerCancel={onPointerEnd}
          onWheel={onWheel}
          // Right-click belongs to the device (Android's context menus).
          onContextMenu={(event) => event.preventDefault()}
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
        ) : locked ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-neutral-950/80 text-center">
            <Lock className="size-6 text-neutral-300" />
            <p className="text-sm font-medium text-neutral-200">Unlock your phone</p>
            <p className="max-w-sm px-6 text-sm text-neutral-400">
              Android shows the desktop only while the phone is unlocked. It appears here as soon as
              you unlock.
            </p>
          </div>
        ) : null}
      </div>
    </div>
  );
}
