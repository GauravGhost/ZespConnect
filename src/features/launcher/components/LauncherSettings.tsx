import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { errorMessage, ipc } from "@/lib/ipc";
import { IS_MAC } from "@/lib/platform";
import type { LauncherSettings as Settings, LauncherStatus } from "@/types";
import { formatShortcut, shortcutFromKeyEvent } from "../shortcut";

const STATUS_KEY = ["launcher-status"];

/** The App launcher row on the Features page: its hotkey and an on/off switch. */
export function LauncherSettings() {
  const queryClient = useQueryClient();
  const { data: status } = useQuery({ queryKey: STATUS_KEY, queryFn: ipc.launcher.status });
  const save = useMutation({
    mutationFn: (settings: Settings) => ipc.launcher.save(settings),
    onSuccess: (next) => queryClient.setQueryData(STATUS_KEY, next),
    onError: (error) => toast.error(errorMessage(error)),
  });

  if (!status) return null;
  const update = (change: Partial<Settings>) =>
    save.mutate({ enabled: status.enabled, shortcut: status.shortcut, ...change });

  return (
    <Section title="App launcher">
      <FeatureRow
        icon={Search}
        label="App launcher"
        hint={launcherHint(status)}
        busy={save.isPending}
        htmlFor="feature-app-launcher"
      >
        <ShortcutButton
          shortcut={status.shortcut}
          disabled={save.isPending}
          onRecord={(shortcut) => update({ shortcut })}
        />
        <Switch
          id="feature-app-launcher"
          checked={status.enabled}
          disabled={save.isPending}
          onCheckedChange={(enabled) => update({ enabled })}
        />
      </FeatureRow>
    </Section>
  );
}

function launcherHint(status: LauncherStatus): string {
  if (status.enabled && !status.registered) return "Hotkey in use by another app";
  return `Press ${formatShortcut(status.shortcut, IS_MAC)} to open a phone app`;
}

/** Shows the hotkey; clicked, it records the next valid key combination. Esc cancels. */
function ShortcutButton({
  shortcut,
  disabled,
  onRecord,
}: Readonly<{ shortcut: string; disabled: boolean; onRecord: (shortcut: string) => void }>) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    function onKeyDown(event: KeyboardEvent) {
      event.preventDefault();
      if (event.code === "Escape") {
        setRecording(false);
        return;
      }
      const recorded = shortcutFromKeyEvent(event, IS_MAC);
      if (!recorded) return;
      setRecording(false);
      onRecord(recorded);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [recording, onRecord]);

  return (
    <Button variant="outline" size="sm" disabled={disabled} onClick={() => setRecording(true)}>
      {recording ? "Press keys" : formatShortcut(shortcut, IS_MAC)}
    </Button>
  );
}
