import { AlertTriangle, Gamepad2, Loader2, Play, RefreshCw, Smartphone } from "lucide-react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useAvds, useStartAvd } from "../hooks";

export function EmulatorsPanel() {
  const { data: avds = [], isLoading, isError, error, isFetching, refetch } = useAvds();
  const start = useStartAvd();

  return (
    <div className="mx-auto w-full max-w-3xl space-y-4 p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold">Emulators</h1>
          <p className="text-muted-foreground text-sm">Android Virtual Devices on this computer.</p>
        </div>
        <Button variant="ghost" size="sm" className="gap-2" onClick={() => refetch()}>
          <RefreshCw className={cn("size-4", isFetching && "animate-spin")} />
          Refresh
        </Button>
      </div>

      {isLoading ? (
        <div className="flex h-40 items-center justify-center">
          <Loader2 className="text-muted-foreground size-5 animate-spin" />
        </div>
      ) : isError ? (
        <Alert variant="destructive">
          <AlertTriangle />
          <AlertTitle>Could not list emulators</AlertTitle>
          <AlertDescription>
            {errorMessage(error)} Install the Android SDK emulator, or set ANDROID_HOME to your SDK.
          </AlertDescription>
        </Alert>
      ) : avds.length === 0 ? (
        <EmptyState
          icon={Gamepad2}
          title="No virtual devices found"
          description="Create an AVD in Android Studio's Device Manager, then refresh."
        />
      ) : (
        <ul className="space-y-2">
          {avds.map((avd) => {
            const busy = start.isPending && start.variables === avd.name;
            return (
              <li
                key={avd.name}
                className="flex items-center gap-3 rounded-2xl border border-border/60 p-3"
              >
                <span className="bg-muted/60 text-muted-foreground flex size-10 shrink-0 items-center justify-center rounded-2xl">
                  <Smartphone className="size-5" />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="truncate text-sm font-medium">{avd.name}</p>
                    {avd.running ? <Badge variant="success">Running</Badge> : null}
                  </div>
                  <p className="text-muted-foreground text-xs">
                    {avd.running ? "Booted and connected" : "Not running"}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="shrink-0 gap-2"
                  disabled={avd.running || busy}
                  onClick={() => start.mutate(avd.name)}
                >
                  {busy ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : (
                    <Play className="size-3.5" />
                  )}
                  Start
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
