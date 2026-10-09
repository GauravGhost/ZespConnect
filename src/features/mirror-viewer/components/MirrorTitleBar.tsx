import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { useWindowTitle } from "../hooks";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { WindowControls } from "./WindowControls";

export function MirrorTitleBar() {
  const title = useWindowTitle();

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
