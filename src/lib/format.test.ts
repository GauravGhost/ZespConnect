import { describe, expect, it } from "vitest";

import { deviceIdentity, fileStamp, formatElapsed, stateLabel, transportLabel } from "@/lib/format";
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
    mdns_name: null,
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

describe("fileStamp", () => {
  it("formats local time for file names", () => {
    expect(fileStamp(new Date(2026, 10, 4, 9, 5, 7))).toBe("2026-11-04_09-05-07");
  });
});

describe("formatElapsed", () => {
  it("shows minutes and seconds", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(42_000)).toBe("0:42");
    expect(formatElapsed(3_725_000)).toBe("62:05");
  });
});
