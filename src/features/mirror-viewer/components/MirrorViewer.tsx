import { Loader2, Lock, MonitorX } from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent,
} from "react";

import { FileDropOverlay } from "@/components/app/FileDropOverlay";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { TouchAction } from "@/types";
import { useKeyboardForwarding, useMirrorStream } from "../hooks";
import { androidButton, androidButtons, wheelNotches } from "../keymap";
import { useRecording } from "../recording";
import { CaptureButtons } from "./CaptureButtons";
import { DesktopTitleBar } from "./DesktopTitleBar";
import { MirrorControlBar } from "./MirrorControlBar";
import { MirrorTitleBar } from "./MirrorTitleBar";

/** Mouse buttons sent with a desktop pointer event (Android `BUTTON_*` masks). */
interface MouseButtons {
  button: number;
  buttons: number;
}

export function MirrorViewer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const activePointerRef = useRef<number | null>(null);
  const moveFrameRef = useRef<number | null>(null);
  const pendingMoveRef = useRef<{ x: number; y: number; hover: boolean; buttons: number } | null>(
    null,
  );
  const sendQueueRef = useRef<Promise<void>>(Promise.resolve());
  const pressedButtonRef = useRef(0);
  const wheelRef = useRef<{ x: number; y: number; dx: number; dy: number } | null>(null);
  const wheelFrameRef = useRef<number | null>(null);
  const {
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
  } = useMirrorStream(canvasRef);
  const serial = session?.serial ?? null;
  // An app's Back is in its title bar, so its window has a small gap instead of a control bar.
  const controlBar = !desktop && session?.kind !== "app";
  const recording = useRecording(sessionId);
  const capture = serial ? (
    <CaptureButtons
      serial={serial}
      canvasRef={canvasRef}
      live={status === "live"}
      recording={recording}
    />
  ) : null;

  useEffect(
    () => () => {
      if (moveFrameRef.current !== null) cancelAnimationFrame(moveFrameRef.current);
      if (wheelFrameRef.current !== null) cancelAnimationFrame(wheelFrameRef.current);
    },
    [],
  );

  /** Queue one input message; all input goes through this chain to keep device order. */
  const enqueue = useCallback(
    (task: (id: number) => Promise<void>) => {
      const id = sessionId;
      if (id === null) return;
      // A dropped input event is not worth surfacing, and must not break the chain.
      sendQueueRef.current = sendQueueRef.current.then(() => task(id)).catch(() => {});
    },
    [sessionId],
  );

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
    if (!interactiveRef.current) {
      send("move", point.x, point.y);
    } else if (point.hover) {
      send("hover", point.x, point.y, { button: 0, buttons: 0 });
    } else {
      send("move", point.x, point.y, { button: 0, buttons: point.buttons });
    }
  }, [send, interactiveRef]);

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

  useKeyboardForwarding(interactive, enqueue);

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
    const id = sessionId;
    if (id === null) return;
    const on = !phoneScreen;
    setPhoneScreen(on);
    ipc.mirror.phoneScreen(id, on).catch(() => setPhoneScreen(!on));
  };

  /** Interactive windows forward real mouse buttons; a screen mirror only a left click. */
  const interactiveMouse = (event: ReactPointerEvent<HTMLCanvasElement>) =>
    interactiveRef.current && event.pointerType === "mouse";

  const onPointerDown = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (activePointerRef.current !== null) return;
    const mouse = interactiveMouse(event);
    // A touch-only screen mirror takes right-click as Back (or wake), like scrcpy.
    if (session?.kind === "mirror" && event.pointerType === "mouse" && event.button === 2) {
      enqueue((id) => ipc.mirror.action(id, "back"));
      return;
    }
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
    const hover = activePointerRef.current === null && interactiveMouse(event);
    if (!hover && activePointerRef.current !== event.pointerId) return;
    const point = pointFor(event);
    if (!point) return;
    pendingMoveRef.current = { ...point, hover, buttons: androidButtons(event.buttons) };
    if (moveFrameRef.current === null) moveFrameRef.current = requestAnimationFrame(flushMove);
  };

  const onWheel = (event: ReactWheelEvent<HTMLCanvasElement>) => {
    if (!interactiveRef.current) return;
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
    if (interactiveMouse(event)) {
      const button = pressedButtonRef.current;
      pressedButtonRef.current = 0;
      send("up", point.x, point.y, { button, buttons: 0 });
    } else {
      send("up", point.x, point.y);
    }
  };

  return (
    <div className="relative flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      {desktop ? (
        <DesktopTitleBar
          serial={serial}
          phoneScreen={phoneScreen}
          onTogglePhoneScreen={togglePhoneScreen}
          capture={capture}
          recording={recording.recording}
        />
      ) : (
        <MirrorTitleBar session={session} capture={capture} />
      )}
      {/* A phone window keeps a control bar (an app a small gap) below rounded corners; a
          desktop fills the area under its title bar. Sized by `mirror_window::vertical_inset`. */}
      <div
        className={cn(
          "relative flex min-h-0 flex-1 items-center justify-center",
          session?.kind === "app" && "pb-2",
        )}
      >
        <canvas
          ref={canvasRef}
          className={cn("max-h-full max-w-full touch-none", !desktop && "rounded-[10px]")}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerEnd}
          onPointerCancel={onPointerEnd}
          onWheel={onWheel}
          // Right-click belongs to the device (Android's context menus).
          onContextMenu={(event) => event.preventDefault()}
        />
        {status !== "live" ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-background/85 text-center">
            {status === "error" ? (
              <>
                <MonitorX className="size-6 text-muted-foreground" />
                <p className="max-w-sm px-6 text-sm text-foreground">{message}</p>
              </>
            ) : (
              <>
                <Loader2 className="size-6 animate-spin text-primary" />
                <p className="text-sm text-muted-foreground">Starting mirror…</p>
              </>
            )}
          </div>
        ) : locked ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-background/85 text-center">
            <Lock className="size-6 text-primary" />
            <p className="text-sm font-medium text-foreground">Unlock your phone</p>
            <p className="max-w-sm px-6 text-sm text-muted-foreground">
              Android shows the desktop only while the phone is unlocked. It appears here as soon as
              you unlock.
            </p>
          </div>
        ) : null}
      </div>
      {controlBar ? (
        <MirrorControlBar key={session?.id} session={session} recording={recording.recording} />
      ) : null}
      <FileDropOverlay serial={serial} />
    </div>
  );
}
