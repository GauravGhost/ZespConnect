import { Activity, AppWindow, Pause, Play } from "lucide-react";
import { useState } from "react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { Section } from "@/components/app/FeatureSection";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useActiveDevice } from "@/hooks/useDevices";
import { deviceName } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePerformance } from "../hooks";
import { HISTORY_LENGTH, seriesStats, type TimedSample } from "../history";
import { METRICS, metricById, type MetricId } from "../metrics";
import { LineChart } from "./LineChart";
import { VitalTile } from "./VitalTile";

const FOCUS_HEIGHT = 180;

function LiveStatus({ live, waiting }: { live: boolean; waiting: boolean }) {
  const label = !live ? "Paused" : waiting ? "Connecting…" : "Live";
  return (
    <span className="bg-muted/70 inline-flex items-center gap-2 rounded-full px-3 py-1 text-xs font-medium">
      <span className="relative flex size-2">
        {live && !waiting ? (
          <span className="bg-primary absolute inline-flex size-full animate-ping rounded-full opacity-60" />
        ) : null}
        <span
          className={cn(
            "relative inline-flex size-2 rounded-full",
            live ? "bg-primary" : "bg-muted-foreground",
          )}
        />
      </span>
      {label}
    </span>
  );
}

function FocusChart({ metricId, history }: { metricId: MetricId; history: TimedSample[] }) {
  const metric = metricById(metricId);
  const values = history.map(metric.value);
  const present = values.filter((v): v is number => v !== null);
  const stats = seriesStats(values);
  const latest = history[history.length - 1];
  const minutes = HISTORY_LENGTH / 60;

  return (
    <Section
      title={`${metric.label} · last ${minutes} minutes`}
      action={
        stats ? (
          <div className="text-muted-foreground flex gap-3 text-xs tabular-nums">
            <span>
              Min <span className="text-foreground font-medium">{metric.format(stats.min)}</span>
            </span>
            <span>
              Avg <span className="text-foreground font-medium">{metric.format(stats.avg)}</span>
            </span>
            <span>
              Max <span className="text-foreground font-medium">{metric.format(stats.max)}</span>
            </span>
          </div>
        ) : null
      }
    >
      <LineChart
        values={values}
        times={history.map((s) => s.at)}
        domain={metric.domain(present, latest)}
        format={metric.format}
        label={`${metric.label} over the last ${minutes} minutes`}
        height={FOCUS_HEIGHT}
        detailed
      />
    </Section>
  );
}

/** One bar per core, so a single pegged core stands out even when the average looks calm. */
function CoresCard({ cores }: { cores: (number | null)[] }) {
  if (cores.length === 0) return null;
  return (
    <Section title="Cores">
      <div className="flex h-28 items-end gap-2">
        {cores.map((usage, index) => (
          <div key={index} className="flex h-full min-w-0 flex-1 flex-col items-center gap-1.5">
            <span className="text-muted-foreground text-[11px] tabular-nums">
              {usage === null ? "off" : `${Math.round(usage)}%`}
            </span>
            <div className="bg-muted/70 relative w-full flex-1 overflow-hidden rounded-lg">
              <div
                className="bg-primary absolute inset-x-0 bottom-0 rounded-t-[4px] transition-[height] duration-700 ease-standard"
                style={{ height: `${usage ?? 0}%` }}
              />
            </div>
            <span className="text-muted-foreground text-[11px]">{index}</span>
          </div>
        ))}
      </div>
    </Section>
  );
}

export function PerformancePanel() {
  const { device } = useActiveDevice();
  const online = device?.state === "device";
  const { history, paused, setPaused, error, live } = usePerformance(online ? device.serial : null);
  const [focus, setFocus] = useState<MetricId>("cpu");

  if (!device || !online) {
    return (
      <NoDeviceState
        icon={Activity}
        description="Connect an Android device to watch its CPU, memory, frame rate, battery and network live."
      />
    );
  }

  const latest = history[history.length - 1];

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-3 p-4">
      <div className="flex flex-wrap items-center gap-2">
        <LiveStatus live={live} waiting={!latest} />
        <span className="text-sm font-medium">{deviceName(device)}</span>
        {latest?.foreground ? (
          <span className="text-muted-foreground inline-flex min-w-0 items-center gap-1.5 text-xs">
            <AppWindow className="size-3.5 shrink-0" />
            <span className="truncate">{latest.foreground.label} in front</span>
          </span>
        ) : null}
        <Button variant="outline" size="sm" className="ml-auto" onClick={() => setPaused(!paused)}>
          {paused ? <Play /> : <Pause />}
          {paused ? "Resume" : "Pause"}
        </Button>
      </div>

      {error ? (
        <Alert variant="destructive">
          <AlertTitle>Could not read the device</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-3">
        {METRICS.map((metric) => (
          <VitalTile
            key={metric.id}
            metric={metric}
            history={history}
            selected={focus === metric.id}
            onSelect={() => setFocus(metric.id)}
          />
        ))}
      </div>

      <FocusChart metricId={focus} history={history} />
      <CoresCard cores={latest?.cores ?? []} />

      <p className="text-muted-foreground px-1 text-xs">
        Readings pause while this page is closed or the window is hidden. Frame rate counts the
        frames the app in front draws, so games that render on their own surface may read low.
      </p>
    </div>
  );
}
