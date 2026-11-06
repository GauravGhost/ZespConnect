import { describe, expect, it } from "vitest";

import type { PerfSample, ProcessInfo } from "@/types";
import { formatAge, formatBytes, formatRate } from "./format";
import { appendSample, niceCeil, seriesStats, type TimedSample } from "./history";
import { metricById } from "./metrics";
import { nextSort, visibleProcesses } from "./processes";

function sample(overrides: Partial<PerfSample> = {}, at = 0): TimedSample {
  return {
    at,
    cpu: null,
    cores: [],
    memUsedKb: null,
    memTotalKb: null,
    fps: null,
    jank: null,
    refreshHz: null,
    foreground: null,
    batteryLevel: null,
    batteryTempC: null,
    batteryMa: null,
    batteryWatts: null,
    charging: null,
    netRxBps: null,
    netTxBps: null,
    ...overrides,
  };
}

describe("appendSample", () => {
  it("keeps only the newest samples", () => {
    let history: TimedSample[] = [];
    for (let at = 0; at < 5; at += 1) history = appendSample(history, sample({}, at), 3);
    expect(history.map((s) => s.at)).toEqual([2, 3, 4]);
  });
});

describe("seriesStats", () => {
  it("skips missing values", () => {
    expect(seriesStats([null, 10, 30, null, 20])).toEqual({ min: 10, max: 30, avg: 20 });
  });

  it("is null without values", () => {
    expect(seriesStats([null, null])).toBeNull();
  });
});

describe("niceCeil", () => {
  it("rounds up to 1, 2 or 5 times a power of ten", () => {
    expect(niceCeil(0)).toBe(1);
    expect(niceCeil(3.4)).toBe(5);
    expect(niceCeil(140)).toBe(200);
    expect(niceCeil(6000)).toBe(10000);
    expect(niceCeil(2)).toBe(2);
  });
});

describe("metrics", () => {
  it("turns memory into a used share", () => {
    const memory = metricById("memory");
    const s = sample({ memUsedKb: 3 * 1024 * 1024, memTotalKb: 8 * 1024 * 1024 });
    expect(memory.value(s)).toBe(37.5);
    expect(memory.detail(s)).toBe("3.0 GB of 8.0 GB");
  });

  it("scales the frame rate chart to the display refresh rate", () => {
    const fps = metricById("fps");
    expect(fps.domain([40, 58], sample({ refreshHz: 120.0001 }))).toEqual([0, 120]);
    expect(fps.domain([], undefined)).toEqual([0, 60]);
  });

  it("flags a hot battery", () => {
    const level = metricById("temperature").level;
    expect(level?.(31)).toBe("normal");
    expect(level?.(41)).toBe("warning");
    expect(level?.(46)).toBe("critical");
  });
});

describe("format", () => {
  it("formats sizes and rates", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(300 * 1024 * 1024)).toBe("300 MB");
    expect(formatRate(2048)).toBe("2.0 KB/s");
  });

  it("formats ages", () => {
    expect(formatAge(200)).toBe("now");
    expect(formatAge(12_000)).toBe("12s ago");
    expect(formatAge(75_000)).toBe("1m 15s ago");
  });
});

function proc(overrides: Partial<ProcessInfo>): ProcessInfo {
  return {
    pid: 1,
    user: "root",
    cpu: 0,
    mem: 0,
    resKb: 0,
    state: "S",
    name: "init",
    package: null,
    label: null,
    ...overrides,
  };
}

describe("visibleProcesses", () => {
  const rows = [
    proc({ pid: 10, name: "system_server", user: "system", cpu: 12, resKb: 500 }),
    proc({ pid: 20, name: "com.whatsapp", package: "com.whatsapp", label: "WhatsApp", cpu: 4 }),
    proc({ pid: 30, name: "com.maps:loc", package: "com.maps", label: "Maps", cpu: 4, resKb: 900 }),
  ];

  it("keeps apps only when asked", () => {
    const result = visibleProcesses(rows, "", "apps", { key: "name", descending: false });
    expect(result.map((p) => p.pid)).toEqual([30, 20]);
  });

  it("matches the label, the command or the exact pid", () => {
    const sort = { key: "pid", descending: false } as const;
    expect(visibleProcesses(rows, "whats", "all", sort).map((p) => p.pid)).toEqual([20]);
    expect(visibleProcesses(rows, ":loc", "all", sort).map((p) => p.pid)).toEqual([30]);
    expect(visibleProcesses(rows, "10", "all", sort).map((p) => p.pid)).toEqual([10]);
  });

  it("breaks ties by pid so rows stay put", () => {
    const result = visibleProcesses(rows, "", "all", { key: "cpu", descending: true });
    expect(result.map((p) => p.pid)).toEqual([10, 30, 20]);
  });
});

describe("nextSort", () => {
  it("flips the current column and starts new ones in their natural order", () => {
    expect(nextSort({ key: "cpu", descending: true }, "cpu")).toEqual({
      key: "cpu",
      descending: false,
    });
    expect(nextSort({ key: "cpu", descending: true }, "name")).toEqual({
      key: "name",
      descending: false,
    });
    expect(nextSort({ key: "name", descending: false }, "memory")).toEqual({
      key: "memory",
      descending: true,
    });
  });
});
