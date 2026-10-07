import { useEffect } from "react";

import { useAppStore, type TabId } from "@/app/store";
import { events } from "@/lib/events";

const TAB_IDS: TabId[] = [
  "overview",
  "apps",
  "mirror",
  "controls",
  "wireless",
  "files",
  "console",
  "emulators",
  "settings",
];

function isTabId(value: string): value is TabId {
  return (TAB_IDS as string[]).includes(value);
}

/** Routes system-tray menu clicks into in-app navigation. */
export function useTrayNavigation() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const setSelectedSerial = useAppStore((s) => s.setSelectedSerial);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    const track = (pending: Promise<() => void>) => {
      pending
        .then((unlisten) => {
          if (disposed) unlisten();
          else unlisteners.push(unlisten);
        })
        .catch(() => {
          /* backend not ready yet */
        });
    };

    track(
      events.trayNavigate((event) => {
        if (isTabId(event.payload.tab)) setActiveTab(event.payload.tab);
      }),
    );
    track(
      events.traySelectDevice((event) => {
        setSelectedSerial(event.payload.serial);
        setActiveTab("overview");
      }),
    );

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [setActiveTab, setSelectedSerial]);
}
