import { Package, Search, Smartphone } from "lucide-react";
import { useEffect, useRef, type KeyboardEvent } from "react";

import { Badge } from "@/components/ui/badge";
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
    <div className="bg-popover text-popover-foreground flex h-screen flex-col overflow-hidden rounded-3xl border select-none">
      <div className="p-3">
        <label className="bg-muted/40 focus-within:bg-muted focus-within:ring-ring flex h-12 items-center gap-3 rounded-2xl px-4 transition-[background-color,box-shadow] duration-150 ease-emphasized focus-within:ring-2">
          <Search className="text-muted-foreground size-5 shrink-0" />
          <input
            ref={inputRef}
            autoFocus
            value={launcher.query}
            onChange={(event) => launcher.setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Search apps"
            aria-label="Search apps"
            className="placeholder:text-muted-foreground min-w-0 flex-1 bg-transparent text-lg outline-none"
          />
          <DeviceChip target={launcher.target} />
        </label>
      </div>
      {launcher.error ? (
        <p className="bg-destructive/10 text-destructive mx-3 mb-2 rounded-2xl px-4 py-2 text-sm">
          {launcher.error}
        </p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        <LauncherBody launcher={launcher} />
      </div>
      <LauncherFooter />
    </div>
  );
}

function DeviceChip({ target }: Readonly<{ target: LauncherTarget | null }>) {
  if (target?.kind !== "ready" && target?.kind !== "offline") return null;
  return (
    <Badge variant={target.kind === "ready" ? "default" : "secondary"}>
      <Smartphone />
      {target.name}
    </Badge>
  );
}

const KEY_HINTS = [
  { key: "Enter", action: "open" },
  { key: "Up/Down", action: "move" },
  { key: "Esc", action: "close" },
];

function LauncherFooter() {
  return (
    <div className="text-muted-foreground flex items-center gap-4 border-t px-4 py-2.5 text-xs">
      <span className="text-foreground flex items-center gap-2 font-medium">
        <span className="bg-primary-container text-on-primary-container flex size-5 items-center justify-center rounded-md">
          <Smartphone className="size-3" />
        </span>
        Zesp Connect
      </span>
      <span className="ml-auto flex items-center gap-3">
        {KEY_HINTS.map(({ key, action }) => (
          <span key={key} className="flex items-center gap-1.5">
            <kbd className="bg-muted text-foreground rounded-md px-1.5 py-0.5 font-sans text-[11px] font-medium">
              {key}
            </kbd>
            {action}
          </span>
        ))}
      </span>
    </div>
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
        "flex h-12 w-full items-center gap-3 rounded-2xl px-3 text-left text-sm transition-colors duration-150 ease-emphasized",
        highlighted ? "bg-primary-container text-on-primary-container" : "hover:bg-muted/60",
      )}
    >
      <span
        className={cn(
          "flex size-8 shrink-0 items-center justify-center rounded-xl",
          highlighted ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground",
        )}
      >
        <Package className="size-4" />
      </span>
      <span className="truncate font-medium">{app.label}</span>
      <span
        className={cn(
          "ml-auto truncate font-mono text-xs",
          highlighted ? "text-on-primary-container/70" : "text-muted-foreground",
        )}
      >
        {app.package}
      </span>
    </button>
  );
}

function Notice({ text }: Readonly<{ text: string }>) {
  return <p className="text-muted-foreground px-4 py-6 text-center text-sm">{text}</p>;
}
