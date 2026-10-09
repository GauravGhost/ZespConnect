import { Camera, Loader2, Monitor, MonitorSmartphone, PlugZap, X } from "lucide-react";
import { useState } from "react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useActiveDevice } from "@/hooks/useDevices";
import {
  useOpenWindows,
  useStartCamera,
  useStartMirror,
  useStopAllMirrors,
  useStopOpenWindow,
} from "@/hooks/useMirrors";
import { useToolsStatus } from "@/hooks/useTools";
import { DEFAULT_MIRROR_OPTIONS, KIND_LABEL } from "@/lib/mirror";
import { cn } from "@/lib/utils";
import type { MirrorOptions } from "@/types";
import { MirrorOptionsForm } from "./MirrorOptionsForm";

const MODES = [
  { id: "screen", label: "Screen", hint: "Mirror the display in a window", icon: Monitor },
  { id: "camera", label: "Camera", hint: "Stream a device camera", icon: Camera },
] as const;

export function MirrorPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;
  const { data: tools } = useToolsStatus();
  const { windows } = useOpenWindows();

  const [screenOptions, setScreenOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);
  const [cameraOptions, setCameraOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);
  const [mode, setMode] = useState<"screen" | "camera">("screen");

  const startMirror = useStartMirror();
  const startCamera = useStartCamera();
  const stopWindow = useStopOpenWindow();
  const stopAll = useStopAllMirrors();

  if (!device) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <EmptyState
          icon={PlugZap}
          title="No device connected"
          description="Connect an Android device to mirror its screen or camera."
        />
      </div>
    );
  }

  const scrcpyReady = Boolean(tools?.scrcpy.found);
  const camera = mode === "camera";
  const options = camera ? cameraOptions : screenOptions;
  const starting = camera ? startCamera.isPending : startMirror.isPending;
  const summary = camera
    ? `${options.camera_facing === "front" ? "Front" : "Back"} camera · ${options.camera_zoom ?? 1}× zoom`
    : `${options.resolution ? `${options.resolution}p` : "Native size"} · ${options.fps ?? 60} fps · ${options.bitrate ?? 8} Mbps`;

  function start() {
    if (!serial) return;
    if (camera) startCamera.mutate({ serial, options: cameraOptions });
    else startMirror.mutate({ serial, options: screenOptions });
  }

  return (
    <div className="mx-auto w-full max-w-5xl space-y-3 p-4">
      {!scrcpyReady ? (
        <Alert variant="warning">
          <AlertTitle>scrcpy is required</AlertTitle>
          <AlertDescription>
            Mirroring needs scrcpy. Install it from the Settings screen to continue.
          </AlertDescription>
        </Alert>
      ) : null}

      <div role="radiogroup" aria-label="What to mirror" className="grid grid-cols-2 gap-3">
        {MODES.map(({ id, label, hint, icon: Icon }) => {
          const selected = mode === id;
          return (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={selected}
              onClick={() => setMode(id)}
              className={cn(
                "flex items-center gap-3 rounded-[1.5rem] border p-3 text-left outline-none",
                "transition-all duration-150 ease-emphasized active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                selected
                  ? "bg-primary-container text-on-primary-container border-transparent shadow-sm"
                  : "bg-card hover:bg-muted/60 border-border/60",
              )}
            >
              <span
                className={cn(
                  "flex size-11 shrink-0 items-center justify-center rounded-2xl",
                  selected ? "bg-on-primary-container/15" : "bg-muted text-primary",
                )}
              >
                <Icon className="size-5" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-semibold">{label}</span>
                <span
                  className={cn(
                    "block truncate text-xs",
                    selected ? "text-on-primary-container/75" : "text-muted-foreground",
                  )}
                >
                  {hint}
                </span>
              </span>
              <span
                aria-hidden="true"
                className={cn(
                  "flex size-5 shrink-0 items-center justify-center rounded-full border-2",
                  selected ? "border-on-primary-container" : "border-muted-foreground/50",
                )}
              >
                {selected ? (
                  <span className="bg-on-primary-container size-2.5 rounded-full" />
                ) : null}
              </span>
            </button>
          );
        })}
      </div>

      <Card className="gap-0 py-0">
        <CardContent className="p-5">
          {camera ? (
            <MirrorOptionsForm variant="camera" value={cameraOptions} onChange={setCameraOptions} />
          ) : (
            <MirrorOptionsForm variant="screen" value={screenOptions} onChange={setScreenOptions} />
          )}
        </CardContent>
        <div className="bg-muted/30 flex items-center justify-between gap-3 border-t border-border/60 px-5 py-3">
          <p className="text-muted-foreground min-w-0 truncate text-xs">{summary}</p>
          <Button className="shrink-0" disabled={!scrcpyReady || starting} onClick={start}>
            {starting ? <Loader2 className="animate-spin" /> : camera ? <Camera /> : <Monitor />}
            {camera ? "Start camera" : "Start mirror"}
          </Button>
        </div>
      </Card>

      <Card className="gap-3 py-4">
        <CardHeader className="px-5">
          <div className="flex items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <MonitorSmartphone className="size-4" />
              Open windows
              <Badge variant="secondary">{windows.length}</Badge>
            </CardTitle>
            {windows.length > 0 ? (
              <Button
                variant="outline"
                size="sm"
                disabled={stopAll.isPending}
                onClick={() => stopAll.mutate()}
              >
                Stop all
              </Button>
            ) : null}
          </div>
        </CardHeader>
        <CardContent className="px-5">
          {windows.length === 0 ? (
            <p className="text-muted-foreground text-sm">No mirror windows are open.</p>
          ) : (
            <ul className="divide-y">
              {windows.map((window) => (
                <li key={window.key} className="flex items-center justify-between gap-3 py-1.5">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{window.label || window.serial}</p>
                    <p className="text-muted-foreground truncate font-mono text-xs">
                      {window.serial}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Badge variant="outline">{KIND_LABEL[window.kind]}</Badge>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="Stop window"
                      disabled={stopWindow.isPending}
                      onClick={() => stopWindow.mutate(window)}
                    >
                      <X />
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
