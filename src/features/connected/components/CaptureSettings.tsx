import { useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import type { LucideIcon } from "lucide-react";
import { Camera, Circle, Clipboard, Folder, FolderOpen, Monitor, Smartphone } from "lucide-react";
import { toast } from "sonner";

import { FeatureRow, Section } from "@/components/app/FeatureSection";
import { Button } from "@/components/ui/button";
import { SegmentedChoice } from "@/components/ui/segmented-choice";
import { errorMessage, ipc } from "@/lib/ipc";
import { useCaptureSettings, type RecordingTarget, type ScreenshotTarget } from "../capture";

const SCREENSHOT_TARGETS = [
  { value: "clipboard", label: "Clipboard", icon: Clipboard },
  { value: "folder", label: "PC folder", icon: Monitor },
  { value: "phone", label: "Phone", icon: Smartphone },
] as const satisfies readonly { value: ScreenshotTarget; label: string; icon: LucideIcon }[];

const RECORDING_TARGETS = [
  { value: "folder", label: "PC folder", icon: Monitor },
  { value: "phone", label: "Phone", icon: Smartphone },
] as const satisfies readonly { value: RecordingTarget; label: string; icon: LucideIcon }[];

function FolderRow({
  dir,
  fallback,
  onChange,
}: Readonly<{
  dir: string | null;
  fallback: string | undefined;
  onChange: (dir: string) => void;
}>) {
  const shown = dir ?? fallback ?? "";

  async function pick() {
    const picked = await open({ directory: true, defaultPath: shown || undefined });
    if (typeof picked === "string") onChange(picked);
  }

  return (
    <FeatureRow icon={Folder} label="PC folder" hint={shown || "Loading..."} busy={false}>
      <Button size="sm" variant="outline" className="rounded-full" onClick={() => void pick()}>
        Change
      </Button>
      <Button
        size="sm"
        variant="outline"
        className="rounded-full"
        disabled={!shown}
        onClick={() =>
          ipc.capture.openFolder(shown).catch((error) => toast.error(errorMessage(error)))
        }
      >
        <FolderOpen />
        Open
      </Button>
    </FeatureRow>
  );
}

/** Where the screenshot and record buttons of mirror windows put their results. */
export function CaptureSettings() {
  const settings = useCaptureSettings();
  const { data: defaults } = useQuery({
    queryKey: ["capture-default-dirs"],
    queryFn: ipc.capture.defaultDirs,
    staleTime: Infinity,
  });

  return (
    <>
      <Section title="Screenshots">
        <p className="text-muted-foreground -mt-1 text-xs">
          From the camera button on any mirror window
        </p>
        <div className="flex flex-col gap-2">
          <FeatureRow icon={Camera} label="Send to" hint="Where a screenshot goes" busy={false}>
            <SegmentedChoice
              label="Send screenshots to"
              options={SCREENSHOT_TARGETS}
              value={settings.screenshotTarget}
              onChange={(screenshotTarget) => settings.update({ screenshotTarget })}
            />
          </FeatureRow>
          <FolderRow
            dir={settings.screenshotDir}
            fallback={defaults?.pictures}
            onChange={(screenshotDir) => settings.update({ screenshotDir })}
          />
        </div>
      </Section>
      <Section title="Recordings">
        <p className="text-muted-foreground -mt-1 text-xs">
          From the record button on any mirror window - video with phone sound
        </p>
        <div className="flex flex-col gap-2">
          <FeatureRow
            icon={Circle}
            label="Save to"
            hint="Where a finished recording goes"
            busy={false}
          >
            <SegmentedChoice
              label="Save recordings to"
              options={RECORDING_TARGETS}
              value={settings.recordingTarget}
              onChange={(recordingTarget) => settings.update({ recordingTarget })}
            />
          </FeatureRow>
          <FolderRow
            dir={settings.recordingDir}
            fallback={defaults?.videos}
            onChange={(recordingDir) => settings.update({ recordingDir })}
          />
        </div>
      </Section>
    </>
  );
}
