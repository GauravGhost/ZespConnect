import { Camera, Monitor, MonitorSmartphone, PlugZap, X } from "lucide-react";
import { useState } from "react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useActiveDevice } from "@/features/devices";
import {
  useOpenWindows,
  useStartCamera,
  useStartMirror,
  useStopAllMirrors,
  useStopOpenWindow,
} from "@/hooks/useMirrors";
import { useToolsStatus } from "@/hooks/useTools";
import { DEFAULT_MIRROR_OPTIONS } from "@/lib/mirror";
import type { MirrorKind, MirrorOptions } from "@/types";
import { MirrorOptionsForm } from "./MirrorOptionsForm";

const KIND_LABEL: Record<MirrorKind, string> = {
  mirror: "Screen",
  camera: "Camera",
  app: "App window",
  "screen-app": "App on screen",
};

export function MirrorPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;
  const { data: tools } = useToolsStatus();
  const { windows } = useOpenWindows();

  const [screenOptions, setScreenOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);
  const [cameraOptions, setCameraOptions] = useState<MirrorOptions>(DEFAULT_MIRROR_OPTIONS);

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

  return (
    <div className="mx-auto w-full max-w-3xl space-y-4 p-5">
      <div className="flex items-baseline gap-2">
        <h1 className="text-base font-semibold">Mirror</h1>
        <p className="text-muted-foreground truncate text-sm">
          {device.display_name || device.model || device.serial}
        </p>
      </div>

      {!scrcpyReady ? (
        <Alert variant="warning">
          <AlertTitle>scrcpy is required</AlertTitle>
          <AlertDescription>
            Mirroring needs scrcpy. Install it from the Settings screen to continue.
          </AlertDescription>
        </Alert>
      ) : null}

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1 px-5">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Monitor className="size-4" />
            Screen
          </CardTitle>
          <CardDescription>Mirror the device display in its own window.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3 px-5">
          <MirrorOptionsForm variant="screen" value={screenOptions} onChange={setScreenOptions} />
          <Button
            disabled={!scrcpyReady || startMirror.isPending}
            onClick={() => serial && startMirror.mutate({ serial, options: screenOptions })}
          >
            <Monitor />
            Start mirror
          </Button>
        </CardContent>
      </Card>

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1 px-5">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Camera className="size-4" />
            Camera
          </CardTitle>
          <CardDescription>Stream the device camera in its own window.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3 px-5">
          <MirrorOptionsForm variant="camera" value={cameraOptions} onChange={setCameraOptions} />
          <Button
            disabled={!scrcpyReady || startCamera.isPending}
            onClick={() => serial && startCamera.mutate({ serial, options: cameraOptions })}
          >
            <Camera />
            Start camera
          </Button>
        </CardContent>
      </Card>

      <Card className="gap-4 py-4">
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
