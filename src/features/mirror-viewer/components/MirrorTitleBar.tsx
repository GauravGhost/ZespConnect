import type { ReactNode } from "react";

import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import type { MirrorSession } from "@/types";
import { offersBack } from "../actions";
import { useWindowTitle } from "../hooks";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { FullscreenButton } from "./FullscreenButton";
import { MirrorBackButton } from "./MirrorControlBar";
import { PhoneSoundButton } from "./PhoneSoundButton";
import { WindowControls } from "./WindowControls";

export function MirrorTitleBar({
  session,
  landscape,
  fullscreen,
  overlay = false,
  capture,
}: Readonly<{
  session: MirrorSession | null;
  landscape: boolean;
  fullscreen: boolean;
  /** Float over the video, shown only on hover (full screen and Game Mode's clean capture). */
  overlay?: boolean;
  capture?: ReactNode;
}>) {
  const title = useWindowTitle();

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-7 shrink-0 items-center gap-0.5 pr-1 select-none",
        // Room for the traffic lights, which macOS hides in full screen.
        IS_MAC && !fullscreen ? "pl-[78px]" : "pl-1",
        (fullscreen || overlay) &&
          "absolute inset-x-0 top-0 z-10 bg-background/90 opacity-0 transition-opacity hover:opacity-100",
      )}
    >
      {session && offersBack(session.kind) ? <MirrorBackButton sessionId={session.id} /> : null}
      <span
        data-tauri-drag-region
        className="min-w-0 flex-1 truncate pl-1.5 text-[13px] font-medium text-foreground"
      >
        {title}
      </span>
      {capture}
      {session ? <PhoneSoundButton serial={session.serial} /> : null}
      {session?.kind === "app" && (landscape || fullscreen) ? (
        <FullscreenButton fullscreen={fullscreen} />
      ) : null}
      <AlwaysOnTopButton />
      {IS_MAC ? null : <WindowControls />}
    </div>
  );
}
