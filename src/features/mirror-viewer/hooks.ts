import { Channel } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState, type RefObject } from "react";

import { useBackendEvents } from "@/hooks/useBackendEvents";
import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { MirrorSession } from "@/types";
import { H264Decoder } from "./decoder";
import { androidKeycode, androidMetaState, isTextKey } from "./keymap";
import { mirrorSessionId } from "./session";

/** The window title, read once. */
export function useWindowTitle() {
  const [title, setTitle] = useState("");

  useEffect(() => {
    let disposed = false;
    getCurrentWindow()
      .title()
      .then((value) => {
        if (!disposed) setTitle(value);
      })
      // An empty title bar is acceptable if the window cannot be queried.
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  return title;
}

/** A window state flag, re-read whenever the window resizes (which every state change does). */
export function useWindowFlag(flag: "isFullscreen" | "isMaximized", enabled = true) {
  const [value, setValue] = useState(false);

  useEffect(() => {
    if (!enabled) return;
    const window = getCurrentWindow();
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    const sync = () => {
      window[flag]()
        .then((next) => {
          if (!disposed) setValue(next);
        })
        // Keep the last known state if the window cannot be queried (e.g. while closing).
        .catch(() => undefined);
    };
    sync();
    window
      .onResized(sync)
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [flag, enabled]);

  return value;
}

export type MirrorStatus = "connecting" | "live" | "error";

/** No decoded frame for this long, while data still arrives, means the decoder stalled. */
const STALL_MS = 1500;
/** How often the stall watchdog runs. */
const WATCHDOG_MS = 500;
/** The least time between two key-frame requests to the device. */
const KEYFRAME_REQUEST_MS = 1000;

/** Desktop windows are opened at `index.html?desktop` (see `open_mirror_window`). */
const DESKTOP_WINDOW = new URLSearchParams(window.location.search).has("desktop");

/** Stream this window's mirror session into `canvasRef` and track the session's state. */
export function useMirrorStream(canvasRef: RefObject<HTMLCanvasElement | null>) {
  const [sessionId] = useState(() => mirrorSessionId(getCurrentWebviewWindow().label));
  const liveRef = useRef(false);
  const aspectRef = useRef<number | null>(null);
  // Desktop and app windows forward mouse, wheel and keyboard; a plain screen mirror is touch-only.
  const interactiveRef = useRef(DESKTOP_WINDOW);
  const [status, setStatus] = useState<MirrorStatus>("connecting");
  const [message, setMessage] = useState<string | null>(null);
  // A desktop session fills its window like a monitor and takes mouse and keyboard.
  const [desktop, setDesktop] = useState(DESKTOP_WINDOW);
  // App windows take the same rich input as desktop mode while staying phone-shaped.
  const [interactive, setInteractive] = useState(DESKTOP_WINDOW);
  // Android hides the desktop while the phone is locked (only the wallpaper draws).
  const [locked, setLocked] = useState(false);
  // Desktop mode turns the phone screen off; the side toggle turns it back on.
  const [phoneScreen, setPhoneScreen] = useState(false);
  // Known once the session starts; the sound switch is per device, not per window.
  const [session, setSession] = useState<MirrorSession | null>(null);

  useEffect(() => {
    const id = sessionId;
    if (id === null) {
      setStatus("error");
      setMessage("This window is not a mirror session.");
      return;
    }

    let disposed = false;
    let lastFrameAt = performance.now();
    let lastDataAt = performance.now();
    let lastKeyframeRequestAt = -Infinity;
    const canvas = canvasRef.current;
    // The canvas is opaque, so skipping alpha lets the compositor blend it cheaper.
    const context = canvas?.getContext("2d", { alpha: false }) ?? null;

    const decoder = new H264Decoder({
      onFrame: (frame) => {
        lastFrameAt = performance.now();
        try {
          if (canvas && context) {
            if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
              canvas.width = frame.displayWidth;
              canvas.height = frame.displayHeight;
              // Shape the window to the video aspect ratio on rotation, not on a pure
              // resolution change (which would only fight the user's own resize).
              const aspect = frame.displayWidth / frame.displayHeight;
              if (aspectRef.current === null || Math.abs(aspect - aspectRef.current) > 0.001) {
                aspectRef.current = aspect;
                // Best effort: the window keeps its current shape if fitting fails.
                void ipc.mirror
                  .fitWindow(id, frame.displayWidth, frame.displayHeight)
                  .catch(() => {});
              }
            }
            context.drawImage(frame, 0, 0, frame.displayWidth, frame.displayHeight);
          }
        } finally {
          // Drawing can throw during a resize; an unclosed frame leaks the decoder pool and
          // eventually silences it (the "black screen, no error" symptom).
          frame.close();
        }

        if (!disposed && !liveRef.current) {
          liveRef.current = true;
          setStatus("live");
        }
      },
      onNeedKeyframe: () => {
        // Throttled: one key frame resyncs; repeated resets just restart the encoder.
        const now = performance.now();
        if (now - lastKeyframeRequestAt < KEYFRAME_REQUEST_MS) return;
        lastKeyframeRequestAt = now;
        // A failed request is retried by the next throttled call.
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

    // A stalled decoder emits no error: while data keeps arriving but no frame comes
    // out, rebuild it so it resumes at the next requested key frame.
    const watchdog = window.setInterval(() => {
      const now = performance.now();
      if (disposed) return;
      if (now - lastFrameAt > STALL_MS && now - lastDataAt < STALL_MS) {
        lastFrameAt = now;
        decoder.reset();
      }
    }, WATCHDOG_MS);

    ipc.mirror
      .startStream(id, channel)
      .then((session) => {
        if (disposed) return;
        const kind = session.kind;
        setDesktop(kind === "desktop");
        const rich = kind === "desktop" || kind === "app" || kind === "screen-app";
        interactiveRef.current = rich;
        setInteractive(rich);
        setPhoneScreen(session.phoneScreen);
        setSession(session);
      })
      .catch(async (failure) => {
        if (disposed) return;
        setStatus("error");
        setMessage(errorMessage(failure));
        // This webview cannot render the stream - reopen as an external window.
        // The error above is already shown if that fails too.
        await ipc.mirror.fallback(id).catch(() => undefined);
      });

    return () => {
      disposed = true;
      window.clearInterval(watchdog);
      channel.onmessage = () => {};
      decoder.close();
    };
  }, [canvasRef, sessionId]);

  useBackendEvents(
    (track) => {
      const id = sessionId;
      if (id === null) return;
      track(
        events.mirrorSessionExited(({ payload }) => {
          // The session is gone either way; a failed stop needs no handling.
          if (payload.id === id) void ipc.mirror.stopSession(id).catch(() => undefined);
        }),
      );
      track(
        events.mirrorSessionLocked(({ payload }) => {
          if (payload.id !== id) return;
          setLocked(payload.locked);
          setPhoneScreen(payload.phoneScreen);
        }),
      );
    },
    [sessionId],
  );

  return {
    sessionId,
    status,
    message,
    desktop,
    interactive,
    interactiveRef,
    locked,
    phoneScreen,
    setPhoneScreen,
    session,
  };
}

/** Forward the keyboard while `enabled`: printable characters as text, the rest as keycodes. */
export function useKeyboardForwarding(
  enabled: boolean,
  enqueue: (task: (id: number) => Promise<void>) => void,
) {
  useEffect(() => {
    if (!enabled) return;
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
  }, [enabled, enqueue]);
}
