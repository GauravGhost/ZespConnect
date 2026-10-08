import { CornerDownLeft, Loader2, Terminal } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ShellResult } from "@/types";
import { useShellExec } from "../hooks";

interface Run {
  command: string;
  result: ShellResult;
}

const SUGGESTIONS = [
  "getprop ro.product.model",
  "dumpsys battery | grep level",
  "pm list packages -3",
];

export function ShellView({ serial }: { serial: string }) {
  const exec = useShellExec(serial);
  const [command, setCommand] = useState("");
  const [runs, setRuns] = useState<Run[]>([]);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [runs]);

  function run(event: React.FormEvent) {
    event.preventDefault();
    const value = command.trim();
    if (!value || exec.isPending) return;
    exec.mutate(value, {
      onSuccess: (result) => setRuns((prev) => [...prev, { command: value, result }]),
    });
    setCommand("");
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-auto p-4 font-mono text-xs">
        {runs.length === 0 ? (
          <div className="text-muted-foreground flex h-full flex-col items-center justify-center gap-3 text-center">
            <Terminal className="size-6" />
            <p className="max-w-sm font-sans text-sm">
              Run a command on the device. Output appears here.
            </p>
            <div className="flex flex-wrap justify-center gap-1.5">
              {SUGGESTIONS.map((suggestion) => (
                <button
                  key={suggestion}
                  type="button"
                  onClick={() => setCommand(suggestion)}
                  className="hover:bg-accent rounded-full border px-2.5 py-1 font-mono text-[11px]"
                >
                  {suggestion}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {runs.map((run, index) => (
              <div key={`${run.command}-${index}`} className="space-y-1">
                <p className="text-primary">
                  <span className="text-muted-foreground">$ </span>
                  {run.command}
                </p>
                {run.result.stdout ? (
                  <pre className="whitespace-pre-wrap">{run.result.stdout.replace(/\n$/, "")}</pre>
                ) : null}
                {run.result.stderr ? (
                  <pre className="text-destructive whitespace-pre-wrap">
                    {run.result.stderr.replace(/\n$/, "")}
                  </pre>
                ) : null}
                {run.result.code !== 0 ? (
                  <Badge variant="destructive">exit {run.result.code}</Badge>
                ) : null}
              </div>
            ))}
            <div ref={endRef} />
          </div>
        )}
      </div>

      <form onSubmit={run} className="flex shrink-0 items-center gap-2 border-t p-3">
        <span className="text-muted-foreground pl-1 font-mono text-sm">$</span>
        <Input
          value={command}
          onChange={(event) => setCommand(event.target.value)}
          placeholder="adb shell command, e.g. getprop ro.product.model"
          className="h-9 font-mono"
          autoComplete="off"
          spellCheck={false}
          autoFocus
        />
        <Button type="submit" size="sm" disabled={!command.trim() || exec.isPending}>
          {exec.isPending ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <CornerDownLeft className="size-4" />
          )}
          Run
        </Button>
      </form>
    </div>
  );
}
