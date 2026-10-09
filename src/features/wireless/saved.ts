import { create } from "zustand";
import { persist } from "zustand/middleware";

import { deviceName } from "@/lib/format";
import type { Device } from "@/types";

export interface SavedWirelessDevice {
  serial: string;
  name: string;
  address: string | null;
  lastConnected: number;
}

interface SavedWirelessStore {
  devices: SavedWirelessDevice[];
  remember: (device: Device) => void;
  forget: (serial: string) => void;
}

const MAX_SAVED = 12;

/** Phones that have connected over Wi-Fi before, most recent first. */
export const useSavedWireless = create<SavedWirelessStore>()(
  persist(
    (set) => ({
      devices: [],
      remember: (device) =>
        set((state) => {
          const entry: SavedWirelessDevice = {
            serial: device.serial,
            name: deviceName(device),
            address: device.address,
            lastConnected: Date.now(),
          };
          const rest = state.devices.filter((saved) => saved.serial !== device.serial);
          return { devices: [entry, ...rest].slice(0, MAX_SAVED) };
        }),
      forget: (serial) =>
        set((state) => ({ devices: state.devices.filter((saved) => saved.serial !== serial) })),
    }),
    { name: "zesp-connect-wireless" },
  ),
);
