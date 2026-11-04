import { Smartphone } from "lucide-react";
import type { ReactNode } from "react";

import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { useWindowFlag, useWindowTitle } from "../hooks";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { PhoneSoundButton } from "./PhoneSoundButton";
import { MirrorIconButton } from "./MirrorIconButton";
import { WindowControls } from "./WindowControls";

export function DesktopTitleBar({
  serial,
  phoneScreen,
  onTogglePhoneScreen,
  capture,
  recording = false,
}: Readonly<{
  serial: string | null;
  phoneScreen: boolean;
  onTogglePhoneScreen: () => void;
  capture?: ReactNode;
  recording?: boolean;
}>) {
  const title = useWindowTitle();
  const fullscreen = useWindowFlag("isFullscreen");

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-7 shrink-0 items-center gap-0.5 pr-1 select-none",
        // Room for the traffic lights, which macOS hides in full screen.
        IS_MAC && !fullscreen ? "pl-[78px]" : "pl-3",
        fullscreen &&
          "absolute inset-x-0 top-0 z-10 bg-background/90 opacity-0 transition-opacity hover:opacity-100",
      )}
    >
      <span
        data-tauri-drag-region
        className="min-w-0 flex-1 truncate text-[13px] font-medium text-foreground"
      >
        {title}
      </span>
      <MirrorIconButton
        label={
          phoneScreen
            ? "Turn the phone's screen off (the desktop keeps running)"
            : "Turn the phone's screen on to use the phone too"
        }
        icon={Smartphone}
        pressed={phoneScreen}
        onClick={onTogglePhoneScreen}
      />
      {capture}
      {serial ? <PhoneSoundButton serial={serial} /> : null}
      <AlwaysOnTopButton />
      {IS_MAC ? null : <WindowControls maximizable locked={recording} />}
    </div>
  );
}
