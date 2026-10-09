import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { WindowControls } from "./WindowControls";

const IS_MAC = /Mac/.test(navigator.platform);

export function MirrorTitleBar() {
  const [title, setTitle] = useState("");

  useEffect(() => {
    let disposed = false;
    void getCurrentWindow()
      .title()
      .then((value) => {
        if (!disposed) setTitle(value);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-7 shrink-0 items-center gap-1 pr-1 select-none",
        IS_MAC ? "pl-[78px]" : "pl-3",
      )}
    >
      <span
        data-tauri-drag-region
        className="min-w-0 flex-1 truncate text-[13px] font-semibold text-neutral-300"
      >
        {title}
      </span>
      <AlwaysOnTopButton className="size-5" />
      {IS_MAC ? null : <WindowControls />}
    </div>
  );
}
