import { Camera, Loader2, Monitor, PlugZap, X } from "lucide-react";
import { useState } from "react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { Section } from "@/components/app/FeatureSection";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SegmentedChoice } from "@/components/ui/segmented-choice";
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
import type { MirrorOptions } from "@/types";
import { MirrorOptionsForm } from "./MirrorOptionsForm";
import { PhoneSoundCard } from "./PhoneSoundCard";

type MirrorMode = "screen" | "camera";

const MODES = [
  { value: "screen", label: "Screen", icon: Monitor },
  { value: "camera", label: "Camera", icon: Camera },
] as const;

export function MirrorPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;
  const { data: tools } = useToolsStatus();

  const [screenOptions, setScreenOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);
  const [cameraOptions, setCameraOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);
  const [mode, setMode] = useState<MirrorMode>("screen");

  const startMirror = useStartMirror();
  const startCamera = useStartCamera();

  if (!device) {
    return (
      <NoDeviceState
        icon={PlugZap}
        description="Connect an Android device to mirror its screen or camera."
      />
    );
  }

  const scrcpyReady = Boolean(tools?.scrcpy.found);
  const camera = mode === "camera";
  const starting = camera ? startCamera.isPending : startMirror.isPending;

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

      <Section
        title="Mirror"
        action={
          <div className="flex items-center gap-2">
            <SegmentedChoice
              label="What to mirror"
              options={MODES}
              value={mode}
              onChange={setMode}
            />
            <Button size="sm" disabled={!scrcpyReady || starting} onClick={start}>
              {starting ? <Loader2 className="animate-spin" /> : camera ? <Camera /> : <Monitor />}
              Start
            </Button>
          </div>
        }
      >
        {camera ? (
          <MirrorOptionsForm variant="camera" value={cameraOptions} onChange={setCameraOptions} />
        ) : (
          <MirrorOptionsForm variant="screen" value={screenOptions} onChange={setScreenOptions} />
        )}
      </Section>

      <div className="grid items-start gap-3 md:grid-cols-2">
        {device ? <PhoneSoundCard serial={device.serial} sdk={device.sdk} /> : null}
        <OpenWindowsSection />
      </div>
    </div>
  );
}

function OpenWindowsSection() {
  const { windows } = useOpenWindows();
  const stopWindow = useStopOpenWindow();
  const stopAll = useStopAllMirrors();

  return (
    <Section
      title={`Open windows · ${windows.length}`}
      action={
        windows.length > 0 ? (
          <Button
            variant="ghost"
            size="sm"
            className="h-7"
            disabled={stopAll.isPending}
            onClick={() => stopAll.mutate()}
          >
            Stop all
          </Button>
        ) : null
      }
    >
      {windows.length === 0 ? (
        <p className="text-muted-foreground text-xs">No mirror windows are open.</p>
      ) : (
        <ul className="-my-1 divide-y">
          {windows.map((window) => (
            <li key={window.key} className="flex items-center gap-2 py-1">
              <p className="min-w-0 flex-1 truncate text-sm font-medium">
                {window.label || window.serial}
              </p>
              <Badge variant="outline" className="shrink-0">
                {KIND_LABEL[window.kind]}
              </Badge>
              <Button
                variant="ghost"
                size="icon"
                className="size-7 shrink-0"
                aria-label={`Stop ${window.label || window.serial}`}
                disabled={stopWindow.isPending}
                onClick={() => stopWindow.mutate(window)}
              >
                <X />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
