import { save } from "@tauri-apps/plugin-dialog";
import {
  Check,
  ChevronDown,
  Download,
  Loader2,
  Pause,
  Play,
  Search,
  Square,
  Tag,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { ComboboxInput } from "@/components/ui/combobox-input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useAppList } from "@/features/apps";
import { fileStamp } from "@/lib/format";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useLogcat } from "../hooks";
import { LOG_LEVELS, matchesQuery, type LogLevel, type ParsedLine } from "../logcat";

const LEVEL_COLORS: Record<LogLevel, string> = {
  V: "text-neutral-400",
  D: "text-sky-400",
  I: "text-emerald-400",
  W: "text-amber-400",
  E: "text-rose-400",
  F: "text-rose-500",
};

function LogRow({ line }: { line: ParsedLine }) {
  const tone = line.stderr
    ? "text-rose-400"
    : line.level
      ? LEVEL_COLORS[line.level]
      : "text-neutral-200";
  return <div className={cn("whitespace-pre-wrap", tone)}>{line.text}</div>;
}

/** Package box with the user's installed apps as presets; typed text applies on Enter. */
function AppFilter({
  serial,
  value,
  onApply,
}: {
  serial: string;
  value: string | null;
  onApply: (pkg: string | null) => void;
}) {
  const { data } = useAppList(serial, false);
  const [draft, setDraft] = useState(value ?? "");
  const options = useMemo(
    () => (data?.apps ?? []).map((app) => ({ value: app.package, label: app.label })),
    [data],
  );

  useEffect(() => setDraft(value ?? ""), [value]);

  function apply(next: string) {
    const pkg = next.trim() || null;
    if (pkg !== value) onApply(pkg);
  }

  function change(next: string) {
    setDraft(next);
    if (!next || options.some((option) => option.value === next)) apply(next);
  }

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        apply(draft);
      }}
    >
      <ComboboxInput
        value={draft}
        onValueChange={change}
        options={options}
        placeholder="All apps"
        aria-label="Only show logs from this app"
        className="h-8 w-56 font-mono text-xs"
        autoComplete="off"
        spellCheck={false}
      />
    </form>
  );
}

function LevelMenu({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (level: LogLevel) => void;
}) {
  const current = LOG_LEVELS.find((level) => level.id === value) ?? LOG_LEVELS[0];
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" aria-label="Minimum log level">
          {current.label}
          <ChevronDown className="size-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {LOG_LEVELS.map((level) => (
          <DropdownMenuItem key={level.id} onSelect={() => onChange(level.id)}>
            <span className={cn("w-3 font-mono", LEVEL_COLORS[level.id])}>{level.id}</span>
            <span className="flex-1">{level.label}</span>
            {level.id === current.id ? <Check className="size-4" /> : null}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function LogcatView({ serial }: { serial: string }) {
  const logcat = useLogcat(serial);
  const { lines, streaming, paused, error, filter } = logcat;
  const [tag, setTag] = useState("");
  const [search, setSearch] = useState("");
  const [autoscroll, setAutoscroll] = useState(true);
  const endRef = useRef<HTMLDivElement>(null);

  const visible = useMemo(
    () => lines.filter((line) => matchesQuery(line, { tag, search })),
    [lines, tag, search],
  );
  const hidden = lines.length - visible.length;

  useEffect(() => {
    if (autoscroll) endRef.current?.scrollIntoView({ block: "end" });
  }, [visible, autoscroll]);

  async function exportLog() {
    const path = await save({
      defaultPath: `logcat-${fileStamp(new Date())}.txt`,
      filters: [{ name: "Text", extensions: ["txt", "log"] }],
      title: "Save log",
    });
    if (typeof path !== "string") return;
    try {
      await ipc.console.saveLog(path, visible.map((line) => line.text).join("\n") + "\n");
      toast.success(`Saved ${visible.length} lines`);
    } catch (failure) {
      toast.error(errorMessage(failure));
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b px-4 py-2">
        {streaming ? (
          <Button variant="outline" size="sm" onClick={logcat.stop}>
            <Square className="size-3.5" />
            Stop
          </Button>
        ) : (
          <Button variant="outline" size="sm" onClick={logcat.start}>
            <Play className="size-3.5" />
            Start
          </Button>
        )}
        <AppFilter
          serial={serial}
          value={filter.package}
          onApply={(pkg) => logcat.setFilter({ ...filter, package: pkg })}
        />
        <LevelMenu
          value={filter.minLevel}
          onChange={(level) =>
            logcat.setFilter({ ...filter, minLevel: level === "V" ? null : level })
          }
        />
        <div className="relative">
          <Tag className="text-muted-foreground absolute top-1/2 left-2 size-3.5 -translate-y-1/2" />
          <Input
            value={tag}
            onChange={(event) => setTag(event.target.value)}
            placeholder="Tag"
            aria-label="Filter by tag"
            className="h-8 w-32 pl-7 text-xs"
            spellCheck={false}
          />
        </div>
        <div className="relative">
          <Search className="text-muted-foreground absolute top-1/2 left-2 size-3.5 -translate-y-1/2" />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search"
            aria-label="Search log text"
            className="h-8 w-44 pl-7 text-xs"
            spellCheck={false}
          />
        </div>

        <div className="ml-auto flex items-center gap-1">
          {streaming ? (
            <span className="text-success mr-1 inline-flex items-center gap-1.5 text-xs">
              {paused ? null : <Loader2 className="size-3.5 animate-spin" />}
              {paused ? "Paused" : "Streaming"}
            </span>
          ) : null}
          <Button
            variant="ghost"
            size="icon"
            onClick={() => logcat.setPaused(!paused)}
            disabled={!streaming}
            aria-label={paused ? "Resume view" : "Pause view"}
            title={paused ? "Resume view" : "Pause view"}
          >
            {paused ? <Play /> : <Pause />}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={logcat.clear}
            disabled={lines.length === 0}
            aria-label="Clear"
            title="Clear"
          >
            <Trash2 />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void exportLog()}
            disabled={visible.length === 0}
            aria-label="Save shown lines to a file"
            title="Save shown lines to a file"
          >
            <Download />
          </Button>
          <Label className="ml-1 flex items-center gap-2 text-xs font-normal">
            <Switch checked={autoscroll} onCheckedChange={setAutoscroll} />
            Auto-scroll
          </Label>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto bg-neutral-950 p-3 font-mono text-[11px] leading-relaxed text-neutral-200">
        {error ? (
          <p className="p-2 text-rose-400">{error}</p>
        ) : visible.length === 0 ? (
          <p className="p-2 text-neutral-400">
            {hidden > 0
              ? `No lines match the tag or search (${hidden} hidden).`
              : streaming
                ? "Waiting for log output…"
                : "Press Start to stream the device log."}
          </p>
        ) : (
          <>
            {visible.map((line, index) => (
              <LogRow key={index} line={line} />
            ))}
            <div ref={endRef} />
          </>
        )}
      </div>
    </div>
  );
}
