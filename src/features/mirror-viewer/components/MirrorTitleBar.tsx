import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";
import type { MirrorSession } from "@/types";
import { offersBack } from "../actions";
import { useWindowTitle } from "../hooks";
import { AlwaysOnTopButton } from "./AlwaysOnTopButton";
import { MirrorBackButton } from "./MirrorControlBar";
import { PhoneSoundButton } from "./PhoneSoundButton";
import { WindowControls } from "./WindowControls";

export function MirrorTitleBar({ session }: Readonly<{ session: MirrorSession | null }>) {
  const title = useWindowTitle();

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-7 shrink-0 items-center gap-0.5 pr-1 select-none",
        IS_MAC ? "pl-[78px]" : "pl-1",
      )}
    >
      {session && offersBack(session.kind) ? <MirrorBackButton sessionId={session.id} /> : null}
      <span
        data-tauri-drag-region
        className="min-w-0 flex-1 truncate pl-1.5 text-[13px] font-medium text-foreground"
      >
        {title}
      </span>
      {session ? <PhoneSoundButton serial={session.serial} /> : null}
      <AlwaysOnTopButton />
      {IS_MAC ? null : <WindowControls />}
    </div>
  );
}
