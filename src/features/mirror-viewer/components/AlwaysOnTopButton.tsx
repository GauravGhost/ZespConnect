import { getCurrentWindow } from "@tauri-apps/api/window";
import { Pin } from "lucide-react";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

export function AlwaysOnTopButton({ className }: Readonly<{ className?: string }>) {
  const [pinned, setPinned] = useState(false);

  useEffect(() => {
    let disposed = false;
    void getCurrentWindow()
      .isAlwaysOnTop()
      .then((value) => {
        if (!disposed) setPinned(value);
      })
      // Keep the unpinned default if the window state cannot be read.
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  const toggle = () => {
    const next = !pinned;
    setPinned(next);
    getCurrentWindow()
      .setAlwaysOnTop(next)
      .catch(() => setPinned(!next));
  };

  return (
    <button
      type="button"
      onClick={toggle}
      aria-pressed={pinned}
      title={pinned ? "Stop keeping this window on top" : "Keep this window on top"}
      className={cn(
        "flex items-center justify-center rounded-full transition-colors",
        pinned ? "text-emerald-300 hover:bg-emerald-500/25" : "text-neutral-300 hover:bg-white/10",
        className,
      )}
    >
      <Pin className={cn("size-3.5", pinned && "fill-current")} />
    </button>
  );
}
