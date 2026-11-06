import { Cable, ScrollText, Terminal } from "lucide-react";
import { useState } from "react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useActiveDevice } from "@/hooks/useDevices";
import { LogcatView } from "./LogcatView";
import { PortsView } from "./PortsView";
import { ShellView } from "./ShellView";

export function ConsolePanel() {
  const { device } = useActiveDevice();
  const [tab, setTab] = useState("shell");

  if (!device) {
    return (
      <NoDeviceState
        icon={Terminal}
        description="Connect an Android device to run shell commands, watch logcat and share ports."
      />
    );
  }

  return (
    <Tabs value={tab} onValueChange={setTab} className="h-full min-h-0 gap-0">
      <div className="shrink-0 border-b px-4 py-2.5">
        <TabsList className="h-10">
          <TabsTrigger value="shell">
            <Terminal />
            Shell
          </TabsTrigger>
          <TabsTrigger value="logcat">
            <ScrollText />
            Logcat
          </TabsTrigger>
          <TabsTrigger value="ports">
            <Cable />
            Ports
          </TabsTrigger>
        </TabsList>
      </div>

      {/* Kept mounted so open shells survive a visit to the other tabs. */}
      <TabsContent value="shell" forceMount className="min-h-0 data-[state=inactive]:hidden">
        <ShellView serial={device.serial} />
      </TabsContent>
      <TabsContent value="logcat" className="min-h-0">
        <LogcatView serial={device.serial} />
      </TabsContent>
      <TabsContent value="ports" className="min-h-0">
        <PortsView serial={device.serial} />
      </TabsContent>
    </Tabs>
  );
}
