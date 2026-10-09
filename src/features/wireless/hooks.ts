import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { useDevices } from "@/hooks/useDevices";
import { errorMessage, ipc } from "@/lib/ipc";

import { useSavedWireless } from "./saved";

export const mdnsKey = ["wireless", "mdns"] as const;

/** Record phones under "Saved devices" as they come online over Wi-Fi. */
export function useRememberSavedDevices() {
  const { data: devices = [] } = useDevices();
  const remember = useSavedWireless((state) => state.remember);
  const knownRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const connected = devices.filter((device) => device.transport === "wifi" && device.connected);
    for (const device of connected) {
      if (!knownRef.current.has(device.serial)) remember(device);
    }
    knownRef.current = new Set(connected.map((device) => device.serial));
  }, [devices, remember]);
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
      queryClient.invalidateQueries({ queryKey: ["devices"] });
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
      queryClient.invalidateQueries({ queryKey: ["devices"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Drop a wireless transport. */
export function useDisconnectWireless() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (addr: string) => ipc.wireless.disconnect(addr),
    onSuccess: () => {
      toast.success("Disconnected");
      queryClient.invalidateQueries({ queryKey: ["devices"] });
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
      queryClient.invalidateQueries({ queryKey: ["devices"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}
