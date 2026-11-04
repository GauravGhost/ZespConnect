import { describe, expect, it } from "vitest";

import type { Device } from "@/types";

import { isSavedDevice, type SavedWirelessDevice } from "./saved";

const saved: SavedWirelessDevice = {
  serial: "adb-PIXEL-b._adb-tls-connect._tcp",
  name: "Pixel 8",
  address: "192.168.1.60:37000",
  mdnsName: "adb-PIXEL-b",
  lastConnected: 0,
};

function wifiDevice(overrides: Partial<Device>): Device {
  return {
    serial: "192.168.1.60:39123",
    display_name: "Pixel 8",
    model: "Pixel 8",
    android_version: null,
    sdk: null,
    transport: "wifi",
    state: "device",
    connected: true,
    battery: null,
    charging: null,
    address: "192.168.1.60:39123",
    mdns_name: null,
    ...overrides,
  };
}

describe("isSavedDevice", () => {
  it("recognises the phone on a new port by its mDNS name", () => {
    expect(isSavedDevice(saved, wifiDevice({ mdns_name: "adb-PIXEL-b" }))).toBe(true);
  });

  it("does not match a different phone", () => {
    expect(isSavedDevice(saved, wifiDevice({ mdns_name: "adb-OTHER-a" }))).toBe(false);
  });

  it("matches entries saved before mDNS names by serial", () => {
    const legacy = { ...saved, mdnsName: undefined };
    expect(isSavedDevice(legacy, wifiDevice({ serial: saved.serial, address: null }))).toBe(true);
  });
});
