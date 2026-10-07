import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { useAppStore } from "@/app/store";
import { events } from "@/lib/events";
import { ipc } from "@/lib/ipc";
import type { Device } from "@/types";

export const devicesKey = ["devices"] as const;

/** Live device list: seeded by `list_devices`, kept fresh by `devices:changed`. */
export function useDevices() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    events
      .devicesChanged((event) => {
        queryClient.setQueryData<Device[]>(devicesKey, event.payload);
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* backend not ready yet — polling fallback still applies */
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [queryClient]);

  return useQuery({
    queryKey: devicesKey,
    queryFn: () => ipc.devices.list(),
    refetchInterval: 15_000,
  });
}

/** The active device, kept consistent with the store as the device list changes. */
export function useActiveDevice() {
  const { data: devices = [], isLoading } = useDevices();
  const selectedSerial = useAppStore((s) => s.selectedSerial);
  const setSelectedSerial = useAppStore((s) => s.setSelectedSerial);

  useEffect(() => {
    if (devices.length === 0) {
      if (selectedSerial) setSelectedSerial(null);
      return;
    }
    const exists = devices.some((d) => d.serial === selectedSerial);
    if (!exists) {
      const preferred = devices.find((d) => d.connected) ?? devices[0];
      setSelectedSerial(preferred.serial);
    }
  }, [devices, selectedSerial, setSelectedSerial]);

  const device = devices.find((d) => d.serial === selectedSerial) ?? null;
  return { device, devices, selectedSerial, setSelectedSerial, isLoading };
}

/** Detailed info (model/android/battery/quick state) for the active device. */
export function useDeviceInfo(serial: string | null) {
  return useQuery({
    queryKey: ["device-info", serial],
    queryFn: () => ipc.devices.info(serial as string),
    enabled: Boolean(serial),
    refetchInterval: 5_000,
  });
}
