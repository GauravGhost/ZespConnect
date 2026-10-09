import { Smartphone } from "lucide-react";

import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { useWindowFlag, useWindowTitle } from "../hooks";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { WindowControls } from "./WindowControls";

export function DesktopTitleBar({
  phoneScreen,
  onTogglePhoneScreen,
}: Readonly<{
  phoneScreen: boolean;
  onTogglePhoneScreen: () => void;
}>) {
  const title = useWindowTitle();
  const fullscreen = useWindowFlag("isFullscreen");

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-7 shrink-0 items-center gap-3 pr-2 select-none",
        // Room for the traffic lights, which macOS hides in full screen.
        IS_MAC && !fullscreen ? "pl-[78px]" : "pl-3",
        fullscreen &&
          "absolute inset-x-0 top-0 z-10 bg-neutral-950/90 opacity-0 transition-opacity hover:opacity-100",
      )}
    >
      <span
        data-tauri-drag-region
        className="min-w-0 flex-1 truncate text-[13px] font-semibold text-neutral-300"
      >
        {title}
      </span>
      <button
        type="button"
        onClick={onTogglePhoneScreen}
        aria-pressed={phoneScreen}
        title={
          phoneScreen
            ? "Turn the phone's screen off (the desktop keeps running)"
            : "Turn the phone's screen on to use the phone too"
        }
        className={cn(
          "flex h-5 items-center gap-1.5 rounded-full px-4 text-[11px] font-medium transition-colors",
          phoneScreen
            ? "text-emerald-300 hover:bg-emerald-500/25"
            : "text-neutral-300 hover:bg-white/10",
        )}
      >
        <Smartphone className="size-3.5" />
      </button>
      <AlwaysOnTopButton className="h-5 w-6" />
      {IS_MAC ? null : <WindowControls maximizable />}
    </div>
  );
}
