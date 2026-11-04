import {
  ArrowLeft,
  Bell,
  ChevronsUp,
  Flashlight,
  RotateCw,
  SlidersHorizontal,
  ZoomIn,
  ZoomOut,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { MirrorAction, MirrorSession } from "@/types";
import { menuActions, type MenuAction } from "../actions";
import { MirrorIconButton } from "./MirrorIconButton";

const ITEMS: Record<MenuAction, { label: string; icon: LucideIcon }> = {
  notifications: { label: "Notifications", icon: Bell },
  "quick-settings": { label: "Quick settings", icon: SlidersHorizontal },
  "collapse-panels": { label: "Close panels", icon: ChevronsUp },
  rotate: { label: "Rotate", icon: RotateCw },
  "zoom-in": { label: "Zoom in", icon: ZoomIn },
  "zoom-out": { label: "Zoom out", icon: ZoomOut },
};

/** Pill-shaped like the active indicator of Android's navigation bar. */
const BAR_BUTTON = "h-7 w-12";

function run(id: number, action: MirrorAction) {
  return ipc.mirror.action(id, action).catch((error) => {
    toast.error(errorMessage(error));
    throw error;
  });
}

/** Android's Back, shown at the left of the title bar. */
export function MirrorBackButton({ sessionId }: Readonly<{ sessionId: number }>) {
  return (
    <MirrorIconButton
      label="Back"
      icon={ArrowLeft}
      // The toast already reports a failure.
      onClick={() => void run(sessionId, "back").catch(() => undefined)}
    />
  );
}

/**
 * Device controls spread along the strip below the video, like Android's navigation bar. Every
 * phone window but an app's has it (its height is `mirror_window::CONTROL_BAR`), so it stays
 * even when a session has no controls.
 */
export function MirrorControlBar({
  session,
  recording = false,
}: Readonly<{ session: MirrorSession | null; recording?: boolean }>) {
  const [torch, setTorch] = useState(session?.torch ?? false);
  const actions = session ? menuActions(session) : [];
  const hasTorch = session?.kind === "camera" && session.cameraControls;

  const toggleTorch = () => {
    if (!session) return;
    const on = !torch;
    setTorch(on);
    run(session.id, on ? "torch-on" : "torch-off").catch(() => setTorch(!on));
  };

  return (
    <div className="flex h-9 shrink-0 items-center justify-around px-2">
      {hasTorch ? (
        <MirrorIconButton
          label="Torch"
          icon={Flashlight}
          pressed={torch}
          onClick={toggleTorch}
          className={BAR_BUTTON}
          iconClassName="size-4"
        />
      ) : null}
      {session
        ? actions.map((action) => (
            <MirrorIconButton
              key={action}
              {...ITEMS[action]}
              // A recording keeps one picture size (see `runtime::recording`).
              {...(action === "rotate" && recording
                ? { label: "Rotate (off while recording)", disabled: true }
                : null)}
              className={BAR_BUTTON}
              iconClassName="size-4"
              // The toast already reports a failure.
              onClick={() => void run(session.id, action).catch(() => undefined)}
            />
          ))
        : null}
    </div>
  );
}
