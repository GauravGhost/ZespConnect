import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";

import { cn } from "@/lib/utils";
import { useWindowFlag } from "../hooks";

const BUTTON =
  "flex size-5 items-center justify-center rounded-full text-neutral-300 transition-colors";

/** Minimize, maximize and close buttons for self-drawn windows; hidden where native controls exist. */
export function WindowControls({ maximizable = false }: Readonly<{ maximizable?: boolean }>) {
  const maximized = useWindowFlag("isMaximized", maximizable);

  const window = getCurrentWindow();
  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        onClick={() => void window.minimize()}
        aria-label="Minimize"
        title="Minimize"
        className={cn(BUTTON, "hover:bg-white/10")}
      >
        <Minus className="size-3.5" />
      </button>
      {maximizable ? (
        <button
          type="button"
          onClick={() => void window.toggleMaximize()}
          aria-label={maximized ? "Restore" : "Maximize"}
          title={maximized ? "Restore" : "Maximize"}
          className={cn(BUTTON, "hover:bg-white/10")}
        >
          {maximized ? <Copy className="size-3" /> : <Square className="size-3" />}
        </button>
      ) : null}
      <button
        type="button"
        onClick={() => void window.close()}
        aria-label="Close"
        title="Close"
        className={cn(BUTTON, "hover:bg-red-500/80 hover:text-white")}
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}
