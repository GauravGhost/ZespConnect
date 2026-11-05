import { getCurrentWindow } from "@tauri-apps/api/window";
import { Maximize, Minimize } from "lucide-react";
import { useCallback } from "react";

import { useEscapeKey } from "../hooks";
import { MirrorIconButton } from "./MirrorIconButton";

/** Full screen for a landscape app window; Escape puts the window back to its previous size. */
export function FullscreenButton({ fullscreen }: Readonly<{ fullscreen: boolean }>) {
  const setFullscreen = useCallback((next: boolean) => {
    // Leaving full screen returns the window to the size the OS saved on the way in.
    void getCurrentWindow()
      .setFullscreen(next)
      .catch(() => undefined);
  }, []);
  // Escape leaves full screen here instead of reaching the app, where it would press Back.
  const exit = useCallback(() => setFullscreen(false), [setFullscreen]);
  useEscapeKey(fullscreen, exit);

  return (
    <MirrorIconButton
      label={fullscreen ? "Exit full screen (Esc)" : "Full screen"}
      icon={fullscreen ? Minimize : Maximize}
      pressed={fullscreen}
      onClick={() => setFullscreen(!fullscreen)}
    />
  );
}
