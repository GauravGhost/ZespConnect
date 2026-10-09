import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

const BUTTON =
  "flex size-5 items-center justify-center rounded-full text-neutral-300 transition-colors";

/**
 * Minimize, maximize and close buttons for a window that draws its own chrome.
 * Only shown where there are no native controls (macOS keeps its traffic lights).
 */
export function WindowControls({ maximizable = false }: Readonly<{ maximizable?: boolean }>) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!maximizable) return;
    const window = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const sync = () =>
      void window.isMaximized().then((value) => {
        if (!disposed) setMaximized(value);
      });
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
  }, [maximizable]);

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
