import { Package, Search } from "lucide-react";
import { useEffect, useRef, type KeyboardEvent } from "react";

import { events } from "@/lib/events";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppInfo, LauncherTarget } from "@/types";
import { useLauncher } from "../hooks";

/** The Spotlight-style popup: search the active phone's apps and open one in its own window. */
export function LauncherPopup() {
  const launcher = useLauncher();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const shown = events.launcherShown(() => inputRef.current?.focus());
    return () => void shown.then((unlisten) => unlisten());
  }, []);

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    const actions: Record<string, () => void> = {
      ArrowDown: () => launcher.move(1),
      ArrowUp: () => launcher.move(-1),
      Enter: () => void launcher.open(),
      Escape: () => void ipc.launcher.hide(),
    };
    const action = actions[event.key];
    if (!action) return;
    event.preventDefault();
    action();
  }

  return (
    <div className="bg-background text-foreground flex h-screen flex-col overflow-hidden border select-none">
      <div className="flex items-center gap-3 border-b px-4 py-3">
        <Search className="text-muted-foreground size-5 shrink-0" />
        <input
          ref={inputRef}
          autoFocus
          value={launcher.query}
          onChange={(event) => launcher.setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Search apps"
          aria-label="Search apps"
          className="placeholder:text-muted-foreground min-w-0 flex-1 bg-transparent text-xl outline-none"
        />
        <DeviceChip target={launcher.target} />
      </div>
      {launcher.error ? (
        <p className="text-destructive border-b px-4 py-2 text-sm">{launcher.error}</p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-y-auto py-1">
        <LauncherBody launcher={launcher} />
      </div>
      <div className="text-muted-foreground flex gap-4 border-t px-4 py-2 text-xs">
        <span>Enter open</span>
        <span>Up/Down move</span>
        <span>Esc close</span>
      </div>
    </div>
  );
}

function DeviceChip({ target }: Readonly<{ target: LauncherTarget | null }>) {
  if (target?.kind !== "ready" && target?.kind !== "offline") return null;
  return (
    <span
      className={cn(
        "shrink-0 rounded-full px-2 py-0.5 text-xs",
        target.kind === "ready" ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground",
      )}
    >
      {target.name}
    </span>
  );
}

function LauncherBody({ launcher }: Readonly<{ launcher: ReturnType<typeof useLauncher> }>) {
  const { target, apps, highlight, loading } = launcher;
  if (!target) return null;
  if (target.kind === "pickInTray") return <Notice text="Pick a phone in the tray menu" />;
  if (target.kind !== "ready") return <Notice text="No phone connected" />;
  if (loading && apps.length === 0) return <Notice text="Loading apps" />;

  return apps.map((app, index) => (
    <AppResult
      key={app.package}
      app={app}
      highlighted={index === highlight}
      onOpen={() => void launcher.open()}
    />
  ));
}

function AppResult({
  app,
  highlighted,
  onOpen,
}: Readonly<{ app: AppInfo; highlighted: boolean; onOpen: () => void }>) {
  const rowRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (highlighted) rowRef.current?.scrollIntoView({ block: "nearest" });
  }, [highlighted]);

  return (
    <button
      ref={rowRef}
      type="button"
      tabIndex={-1}
      onClick={onOpen}
      className={cn(
        "flex w-full items-center gap-3 px-4 py-2 text-left text-sm",
        highlighted && "bg-primary/15",
      )}
    >
      <span className="bg-muted text-muted-foreground flex size-7 shrink-0 items-center justify-center rounded-md">
        <Package className="size-4" />
      </span>
      <span className="truncate font-medium">{app.label}</span>
      <span className="text-muted-foreground ml-auto truncate text-xs">{app.package}</span>
    </button>
  );
}

function Notice({ text }: Readonly<{ text: string }>) {
  return <p className="text-muted-foreground px-4 py-6 text-center text-sm">{text}</p>;
}
