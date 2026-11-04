import { create } from "zustand";
import { persist } from "zustand/middleware";

import { deviceName } from "@/lib/format";
import type { Device, ReconnectTarget } from "@/types";

export interface SavedWirelessDevice {
  serial: string;
  name: string;
  address: string | null;
  /** Missing on entries saved before auto-reconnect; learned on their next connect. */
  mdnsName?: string | null;
  /** The phone forgot this computer, so it is not reconnected until paired again. */
  needsPair?: boolean;
  lastConnected: number;
}

interface SavedWirelessStore {
  devices: SavedWirelessDevice[];
  /** Phones disconnected by hand this session; not persisted, so a restart resumes them. */
  paused: string[];
  remember: (device: Device) => void;
  forget: (serial: string) => void;
  markNeedsPair: (serial: string) => void;
  pause: (serial: string) => void;
  resume: (serial: string) => void;
}

const MAX_SAVED = 12;

/** Whether a saved entry is this device, through any of its transports. */
export function isSavedDevice(saved: SavedWirelessDevice, device: Device): boolean {
  return (
    saved.serial === device.serial ||
    (!!saved.mdnsName && saved.mdnsName === device.mdns_name) ||
    (!!saved.address && saved.address === device.address)
  );
}

export function reconnectTarget(saved: SavedWirelessDevice): ReconnectTarget {
  return { serial: saved.serial, name: saved.mdnsName ?? null, address: saved.address };
}

/** Phones that have connected over Wi-Fi before, most recent first. */
export const useSavedWireless = create<SavedWirelessStore>()(
  persist(
    (set) => ({
      devices: [],
      paused: [],
      remember: (device) =>
        set((state) => {
          const previous = state.devices.find((saved) => isSavedDevice(saved, device));
          const entry: SavedWirelessDevice = {
            serial: device.serial,
            name: deviceName(device),
            address: device.address ?? previous?.address ?? null,
            mdnsName: device.mdns_name ?? previous?.mdnsName ?? null,
            lastConnected: Date.now(),
          };
          const rest = state.devices.filter((saved) => !isSavedDevice(saved, device));
          // Back online, so a pause from an earlier disconnect no longer applies.
          const paused = state.paused.filter(
            (serial) => serial !== device.serial && serial !== previous?.serial,
          );
          return { devices: [entry, ...rest].slice(0, MAX_SAVED), paused };
        }),
      forget: (serial) =>
        set((state) => ({ devices: state.devices.filter((saved) => saved.serial !== serial) })),
      markNeedsPair: (serial) =>
        set((state) => ({
          devices: state.devices.map((saved) =>
            saved.serial === serial ? { ...saved, needsPair: true } : saved,
          ),
        })),
      pause: (serial) =>
        set((state) => ({
          paused: state.paused.includes(serial) ? state.paused : [...state.paused, serial],
        })),
      resume: (serial) =>
        set((state) => ({ paused: state.paused.filter((paused) => paused !== serial) })),
    }),
    {
      name: "zesp-connect-wireless",
      partialize: (state) => ({ devices: state.devices }),
    },
  ),
);
