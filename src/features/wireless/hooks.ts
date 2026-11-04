import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { useBackendEvents } from "@/hooks/useBackendEvents";
import { devicesKey, useDevices } from "@/hooks/useDevices";
import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { Device } from "@/types";

import {
  isSavedDevice,
  reconnectTarget,
  useSavedWireless,
  type SavedWirelessDevice,
} from "./saved";

export const mdnsKey = ["wireless", "mdns"] as const;

/**
 * App-wide: save phones as they come online over Wi-Fi, have the backend watch the saved ones
 * (except phones paused by a disconnect or needing a new pairing), and report what it does.
 */
export function useWirelessReconnect() {
  const { data: devices = [] } = useDevices();
  const saved = useSavedWireless((state) => state.devices);
  const paused = useSavedWireless((state) => state.paused);
  const remember = useSavedWireless((state) => state.remember);
  const knownRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const connected = devices.filter((device) => device.transport === "wifi" && device.connected);
    for (const device of connected) {
      if (!knownRef.current.has(device.serial)) remember(device);
    }
    knownRef.current = new Set(connected.map((device) => device.serial));
  }, [devices, remember]);

  useEffect(() => {
    const targets = saved
      .filter((entry) => !entry.needsPair && !paused.includes(entry.serial))
      .map(reconnectTarget);
    ipc.wireless.setReconnectTargets(targets).catch(() => {
      /* backend not ready yet; the next change resends */
    });
  }, [saved, paused]);

  useBackendEvents((track) => {
    const nameOf = (serial: string) =>
      useSavedWireless.getState().devices.find((entry) => entry.serial === serial)?.name ?? serial;
    track(
      events.wirelessReconnected(({ payload }) => {
        toast.success(`Reconnected ${nameOf(payload.serial)}`);
      }),
    );
    track(
      events.wirelessUnpaired(({ payload }) => {
        useSavedWireless.getState().markNeedsPair(payload.serial);
        toast.error(`${nameOf(payload.serial)} no longer trusts this computer. Pair it again.`);
      }),
    );
  }, []);
}

/** Live mDNS discovery of adb pairing/connect services (polls while mounted). */
export function useMdnsServices() {
  return useQuery({
    queryKey: mdnsKey,
    queryFn: () => ipc.wireless.mdnsServices(),
    refetchInterval: 3_000,
  });
}

/** Pair an already-advertised device with its on-screen code. */
export function usePairCode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ addr, code }: { addr: string; code: string }) =>
      ipc.wireless.pairCode(addr, code),
    onSuccess: (address) => {
      toast.success(`Paired with ${address}`);
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** `adb connect` an address (fresh connect or a reconnect). */
export function useConnectWireless() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (addr: string) => ipc.wireless.connect(addr),
    onSuccess: (address) => {
      toast.success(`Connected to ${address}`);
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Reconnect a saved phone, following it to its current port; resumes auto-reconnect for it. */
export function useReconnectSaved() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (entry: SavedWirelessDevice) => {
      useSavedWireless.getState().resume(entry.serial);
      return ipc.wireless.reconnect(reconnectTarget(entry));
    },
    onSuccess: (address) => {
      toast.success(`Connected to ${address}`);
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Drop a wireless transport; a saved phone disconnected by hand is not auto-reconnected. */
export function useDisconnectWireless() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (device: Device) => {
      const { devices, pause } = useSavedWireless.getState();
      const entry = devices.find((saved) => isSavedDevice(saved, device));
      if (entry) pause(entry.serial);
      return ipc.wireless.disconnect(device.address ?? device.serial);
    },
    onSuccess: () => {
      toast.success("Disconnected");
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Switch a USB-connected device to Wi-Fi and connect to it. */
export function useEnableTcpip() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, port }: { serial: string; port?: number }) =>
      ipc.wireless.enableTcpip(serial, port),
    onSuccess: (address) => {
      toast.success(`Wireless debugging on - connected to ${address}`);
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}
