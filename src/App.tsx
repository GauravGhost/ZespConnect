import { Loader2 } from "lucide-react";

import { AppHeader } from "@/components/app/AppHeader";
import { ComingSoon } from "@/components/app/ComingSoon";
import { Sidebar } from "@/components/app/Sidebar";
import { EmptyState } from "@/components/app/EmptyState";
import { useAppStore, type TabId } from "@/app/store";
import { OverviewPanel } from "@/features/devices";
import { DependencyCheck } from "@/features/onboarding";
import { SettingsPanel } from "@/features/settings";
import { WirelessPanel } from "@/features/wireless";
import { useToolsStatus } from "@/hooks/useTools";
import { useTrayNavigation } from "@/hooks/useTray";

const TAB_TITLES: Record<TabId, string> = {
  overview: "Overview",
  apps: "Apps",
  mirror: "Mirror",
  controls: "Controls",
  wireless: "Wireless",
  files: "Files",
  console: "Console",
  emulators: "Emulators",
  settings: "Settings",
};

function ActiveFeature({ tab }: { tab: TabId }) {
  if (tab === "overview") return <OverviewPanel />;
  if (tab === "wireless") return <WirelessPanel />;
  if (tab === "settings") return <SettingsPanel />;
  return (
    <div className="flex h-full items-center justify-center">
      <ComingSoon title={TAB_TITLES[tab]} />
    </div>
  );
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

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        <main className="min-h-0 flex-1 overflow-y-auto">
          <Main />
        </main>
      </div>
    </div>
  );
}
