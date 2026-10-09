import { Clipboard, ScrollText, Terminal } from "lucide-react";
import { useState } from "react";

import { EmptyState } from "@/components/app/EmptyState";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useActiveDevice } from "@/hooks/useDevices";
import { ClipboardView } from "./ClipboardView";
import { LogcatView } from "./LogcatView";
import { ShellView } from "./ShellView";

export function ConsolePanel() {
  const { device } = useActiveDevice();
  const [tab, setTab] = useState("shell");

  if (!device) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <EmptyState
          icon={Terminal}
          title="No device connected"
          description="Connect an Android device to run shell commands, watch logcat and share the clipboard."
        />
      </div>
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
          <TabsTrigger value="clipboard">
            <Clipboard />
            Clipboard
          </TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="shell" className="min-h-0">
        <ShellView serial={device.serial} />
      </TabsContent>
      <TabsContent value="logcat" className="min-h-0">
        <LogcatView serial={device.serial} />
      </TabsContent>
      <TabsContent value="clipboard" className="min-h-0 overflow-auto">
        <ClipboardView serial={device.serial} />
      </TabsContent>
    </Tabs>
  );
}
