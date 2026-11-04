import { Loader2 } from "lucide-react";

import { AppHeader } from "@/components/app/AppHeader";
import { Sidebar } from "@/components/app/Sidebar";
import { EmptyState } from "@/components/app/EmptyState";
import { FileDropOverlay } from "@/components/app/FileDropOverlay";
import { useAppStore, type TabId } from "@/app/store";
import { AppsPanel } from "@/features/apps";
import { ConsolePanel } from "@/features/console";
import { OverviewPanel } from "@/features/devices";
import { EmulatorsPanel } from "@/features/emulators";
import { FilesPanel } from "@/features/files";
import { MirrorPanel } from "@/features/mirror";
import { DependencyCheck } from "@/features/onboarding";
import { SettingsPanel } from "@/features/settings";
import { useWirelessReconnect, WirelessPanel } from "@/features/wireless";
import { useToolsStatus } from "@/hooks/useTools";
import { useTrayNavigation } from "@/hooks/useTray";

function ActiveFeature({ tab }: { tab: TabId }) {
  if (tab === "overview") return <OverviewPanel />;
  if (tab === "apps") return <AppsPanel />;
  if (tab === "mirror") return <MirrorPanel />;
  if (tab === "wireless") return <WirelessPanel />;
  if (tab === "files") return <FilesPanel />;
  if (tab === "console") return <ConsolePanel />;
  if (tab === "emulators") return <EmulatorsPanel />;
  if (tab === "settings") return <SettingsPanel />;
  return null;
}

function Main() {
  const activeTab = useAppStore((s) => s.activeTab);
  const { data: tools, isLoading } = useToolsStatus();

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <EmptyState
          icon={Loader2}
          title="Starting up…"
          description="Checking for adb and scrcpy."
        />
      </div>
    );
  }

  // adb is the hard dependency; without it nothing device-related works.
  if (tools && !tools.adb.found) {
    return <DependencyCheck />;
  }

  return <ActiveFeature tab={activeTab} />;
}

export default function App() {
  useTrayNavigation();
  useWirelessReconnect();
  const selectedSerial = useAppStore((s) => s.selectedSerial);

  return (
    <div className="relative flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        <main className="min-h-0 flex-1 overflow-y-auto">
          <Main />
        </main>
      </div>
      <FileDropOverlay serial={selectedSerial} />
    </div>
  );
}
