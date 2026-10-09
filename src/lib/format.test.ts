import { describe, expect, it } from "vitest";

import { deviceIdentity, stateLabel, transportLabel } from "@/lib/format";
import type { Device } from "@/types";

function makeDevice(overrides: Partial<Device> = {}): Device {
  return {
    serial: "0123456789ABCDEF",
    display_name: "Pixel 8",
    model: "Pixel 8",
    android_version: "15",
    sdk: "35",
    transport: "usb",
    state: "device",
    connected: true,
    battery: 82,
    charging: true,
    address: null,
    ...overrides,
  };
}

describe("format", () => {
  it("builds the header identity line", () => {
    expect(deviceIdentity(makeDevice())).toBe("Pixel 8 · Android 15 · 82% · USB");
  });

  it("falls back gracefully when fields are missing", () => {
    const device = makeDevice({ display_name: "", android_version: null, battery: null });
    expect(deviceIdentity(device)).toBe("Pixel 8 · USB");
  });

  it("labels transports and states", () => {
    expect(transportLabel(makeDevice({ transport: "wifi" }))).toBe("Wi-Fi");
    expect(transportLabel(makeDevice({ transport: "usb" }))).toBe("USB");
    expect(stateLabel("unauthorized")).toBe("Unauthorized");
    expect(stateLabel("device")).toBe("Connected");
  });
});
