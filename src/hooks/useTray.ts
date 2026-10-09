import { useEffect } from "react";

import { TAB_IDS } from "@/app/navigation";
import { useAppStore, type TabId } from "@/app/store";
import { events } from "@/lib/events";
import { ipc } from "@/lib/ipc";

import { useBackendEvents } from "./useBackendEvents";

function isTabId(value: string): value is TabId {
  return (TAB_IDS as string[]).includes(value);
}

/** Routes system-tray menu clicks into in-app navigation. */
export function useTrayNavigation() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const setSelectedSerial = useAppStore((s) => s.setSelectedSerial);
  const selectedSerial = useAppStore((s) => s.selectedSerial);

  useBackendEvents(
    (track) => {
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
    },
    [setActiveTab, setSelectedSerial],
  );

  // Seed the selection from the backend on first run (so restarts restore it).
  useEffect(() => {
    if (useAppStore.getState().selectedSerial) return;
    let disposed = false;
    ipc.devices
      .selected()
      .then((serial) => {
        if (!disposed && serial) setSelectedSerial(serial);
      })
      .catch(() => {
        /* backend not ready yet */
      });
    return () => {
      disposed = true;
    };
  }, [setSelectedSerial]);

  // Mirror the UI selection to the backend so the tray checkmark follows.
  useEffect(() => {
    if (!selectedSerial) return;
    ipc.devices.setSelected(selectedSerial).catch(() => {
      /* backend not ready yet */
    });
  }, [selectedSerial]);
}
