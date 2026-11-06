import { Bug, LayoutTemplate, Wrench } from "lucide-react";
import { useState } from "react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useActiveDevice } from "@/hooks/useDevices";
import { CrashesView } from "./CrashesView";
import { DeveloperView } from "./DeveloperView";
import { LayoutView } from "./LayoutView";

export function DebuggingPanel() {
  const { device } = useActiveDevice();
  const [tab, setTab] = useState("developer");

  if (!device) {
    return (
      <NoDeviceState
        icon={Bug}
        description="Connect an Android device to change developer options, inspect layouts and read crash reports."
      />
    );
  }

  return (
    <Tabs value={tab} onValueChange={setTab} className="h-full min-h-0 gap-0">
      <div className="shrink-0 border-b px-4 py-2.5">
        <TabsList className="h-10">
          <TabsTrigger value="developer">
            <Wrench />
            Developer options
          </TabsTrigger>
          <TabsTrigger value="layout">
            <LayoutTemplate />
            Layout
          </TabsTrigger>
          <TabsTrigger value="crashes">
            <Bug />
            Crashes
          </TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="developer" className="min-h-0 overflow-y-auto">
        <DeveloperView serial={device.serial} />
      </TabsContent>
      {/* Kept mounted so a capture survives a look at another tab. */}
      <TabsContent value="layout" forceMount className="min-h-0 data-[state=inactive]:hidden">
        <LayoutView serial={device.serial} />
      </TabsContent>
      <TabsContent value="crashes" className="min-h-0">
        <CrashesView serial={device.serial} />
      </TabsContent>
    </Tabs>
  );
}
