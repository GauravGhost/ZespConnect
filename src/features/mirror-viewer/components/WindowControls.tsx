import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";

import { useWindowFlag } from "../hooks";
import { MirrorIconButton } from "./MirrorIconButton";

/** Minimize, maximize and close buttons for self-drawn windows; hidden where native controls exist. */
export function WindowControls({ maximizable = false }: Readonly<{ maximizable?: boolean }>) {
  const maximized = useWindowFlag("isMaximized", maximizable);

  const window = getCurrentWindow();
  return (
    <div className="flex items-center gap-0.5">
      <MirrorIconButton label="Minimize" icon={Minus} onClick={() => void window.minimize()} />
      {maximizable ? (
        <MirrorIconButton
          label={maximized ? "Restore" : "Maximize"}
          icon={maximized ? Copy : Square}
          iconClassName="size-3"
          onClick={() => void window.toggleMaximize()}
        />
      ) : null}
      <MirrorIconButton label="Close" icon={X} danger onClick={() => void window.close()} />
    </div>
  );
}
