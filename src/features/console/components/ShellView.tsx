import { CornerDownLeft, Plus, RotateCcw, Terminal, Trash2, X, OctagonX } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useShellSessions, type ShellTab } from "../hooks";

const SUGGESTIONS = [
  "getprop ro.product.model",
  "dumpsys battery | grep level",
  "pm list packages -3",
];
const INTERRUPT = "\x03";
const MAX_HISTORY = 100;

function TabButton({
  tab,
  active,
  onSelect,
  onClose,
}: {
  tab: ShellTab;
  active: boolean;
  onSelect: () => void;
  onClose: () => void;
}) {
  return (
    <div
      className={cn(
        "flex h-8 items-center rounded-md border text-xs",
        active ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent/50",
      )}
    >
      <button type="button" onClick={onSelect} className="flex h-full items-center gap-1.5 pl-2.5">
        <span
          className={cn(
            "size-1.5 rounded-full",
            tab.status === "running"
              ? "bg-emerald-500"
              : tab.status === "starting"
                ? "bg-amber-400"
                : "bg-neutral-400",
          )}
        />
        {`Shell ${tab.number}`}
      </button>
      <button
        type="button"
        onClick={onClose}
        className="hover:text-foreground flex h-full items-center px-1.5"
        aria-label={`Close Shell ${tab.number}`}
        title="Close"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}

export function ShellView({ serial }: { serial: string }) {
  const shells = useShellSessions(serial);
  const { tabs, activeKey } = shells;
  const tab = tabs.find((item) => item.key === activeKey) ?? null;
  const [command, setCommand] = useState("");
  const [history, setHistory] = useState<string[]>([]);
  // Position while browsing history with the arrow keys; null when editing a fresh line.
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const [usedKeys, setUsedKeys] = useState<Set<number>>(() => new Set());
  const inputRef = useRef<HTMLInputElement>(null);
  const endRef = useRef<HTMLDivElement>(null);

  const running = tab?.status === "running";

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [tab?.output, tab?.status]);

  useEffect(() => {
    if (running) inputRef.current?.focus();
  }, [running, activeKey]);

  function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!tab || !running) return;
    shells.send(tab.key, `${command}\n`);
    const value = command.trim();
    if (value && history[history.length - 1] !== value) {
      setHistory((prev) => [...prev, value].slice(-MAX_HISTORY));
    }
    if (value) setUsedKeys((prev) => new Set(prev).add(tab.key));
    setHistoryIndex(null);
    setCommand("");
  }

  function browseHistory(step: -1 | 1) {
    if (history.length === 0) return;
    const from = historyIndex ?? history.length;
    const next = Math.min(Math.max(from + step, 0), history.length);
    setHistoryIndex(next === history.length ? null : next);
    setCommand(next === history.length ? "" : history[next]);
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      browseHistory(event.key === "ArrowUp" ? -1 : 1);
      return;
    }
    const input = event.currentTarget;
    const selecting = input.selectionStart !== input.selectionEnd;
    if (event.ctrlKey && event.key.toLowerCase() === "c" && !selecting && tab && running) {
      event.preventDefault();
      shells.send(tab.key, INTERRUPT);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-1.5 border-b px-4 py-2">
        {tabs.map((item) => (
          <TabButton
            key={item.key}
            tab={item}
            active={item.key === activeKey}
            onSelect={() => shells.setActiveKey(item.key)}
            onClose={() => shells.closeTab(item.key)}
          />
        ))}
        <Button
          variant="ghost"
          size="icon"
          onClick={shells.openTab}
          aria-label="New shell"
          title="New shell"
        >
          <Plus />
        </Button>

        <div className="ml-auto flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => tab && shells.send(tab.key, INTERRUPT)}
            disabled={!running}
            title="Stop the running command (Ctrl+C)"
          >
            <OctagonX className="size-3.5" />
            Ctrl+C
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => tab && shells.clear(tab.key)}
            disabled={!tab?.output}
            aria-label="Clear"
            title="Clear"
          >
            <Trash2 />
          </Button>
        </div>
      </div>

      {tab ? (
        <div
          className="min-h-0 flex-1 overflow-auto bg-neutral-950 p-3 font-mono text-xs leading-relaxed text-neutral-200"
          onClick={() => {
            if (!window.getSelection()?.toString()) inputRef.current?.focus();
          }}
        >
          <pre className="font-mono break-all whitespace-pre-wrap">{tab.output}</pre>
          {tab.status === "starting" && !tab.output ? (
            <p className="text-neutral-400">Starting shell…</p>
          ) : null}
          {tab.status === "ended" ? (
            <div className="mt-2 flex flex-wrap items-center gap-3 font-sans">
              <span className={tab.error ? "text-rose-400" : "text-neutral-400"}>
                {tab.error ??
                  (tab.exitCode === null
                    ? "Shell closed."
                    : `Shell exited with code ${tab.exitCode}.`)}
              </span>
              <Button variant="outline" size="sm" onClick={() => shells.restart(tab.key)}>
                <RotateCcw className="size-3.5" />
                Restart
              </Button>
            </div>
          ) : null}
          {running && !usedKeys.has(tab.key) ? (
            <div className="mt-3 flex flex-wrap gap-1.5 font-sans">
              {SUGGESTIONS.map((suggestion) => (
                <button
                  key={suggestion}
                  type="button"
                  onClick={() => setCommand(suggestion)}
                  className="rounded-full border border-neutral-700 px-2.5 py-1 font-mono text-[11px] text-neutral-300 hover:bg-neutral-800"
                >
                  {suggestion}
                </button>
              ))}
            </div>
          ) : null}
          <div ref={endRef} />
        </div>
      ) : (
        <div className="text-muted-foreground flex min-h-0 flex-1 flex-col items-center justify-center gap-3 text-center">
          <Terminal className="size-6" />
          <p className="text-sm">No shell open.</p>
          <Button variant="outline" size="sm" onClick={shells.openTab}>
            <Plus className="size-3.5" />
            New shell
          </Button>
        </div>
      )}

      <form onSubmit={submit} className="flex shrink-0 items-center gap-2 border-t p-3">
        <span className="text-muted-foreground pl-1 font-mono text-sm">$</span>
        <Input
          ref={inputRef}
          value={command}
          onChange={(event) => {
            setCommand(event.target.value);
            setHistoryIndex(null);
          }}
          onKeyDown={onKeyDown}
          placeholder={
            running ? "Command, e.g. cd /sdcard (state carries over)" : "Shell is not running"
          }
          className="h-9 font-mono"
          disabled={!running}
          autoComplete="off"
          spellCheck={false}
        />
        <Button type="submit" size="sm" disabled={!running}>
          <CornerDownLeft className="size-4" />
          Run
        </Button>
      </form>
    </div>
  );
}
