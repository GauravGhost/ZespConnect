import { getCurrentWindow } from "@tauri-apps/api/window";
import { Pin } from "lucide-react";
import { useEffect, useState } from "react";

import { MirrorIconButton } from "./MirrorIconButton";

export function AlwaysOnTopButton() {
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
    <MirrorIconButton
      label={pinned ? "Stop keeping this window on top" : "Keep this window on top"}
      icon={Pin}
      iconClassName={pinned ? "fill-current" : undefined}
      pressed={pinned}
      onClick={toggle}
    />
  );
}
