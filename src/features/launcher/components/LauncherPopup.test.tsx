import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import type { AppInfo } from "@/types";
import { LauncherPopup } from "./LauncherPopup";

const launcher = vi.hoisted(() => ({
  target: vi.fn(),
  apps: vi.fn(),
  hide: vi.fn(),
  startAppWindow: vi.fn(),
}));

vi.mock("@/lib/ipc", () => ({
  errorMessage: (error: unknown) => String(error),
  ipc: {
    launcher: { target: launcher.target, apps: launcher.apps, hide: launcher.hide },
    mirror: { startAppWindow: launcher.startAppWindow },
  },
}));

vi.mock("@/lib/events", () => ({
  events: {
    launcherShown: () => Promise.resolve(() => {}),
    launcherAppsChanged: () => Promise.resolve(() => {}),
  },
}));

function makeApp(label: string): AppInfo {
  return {
    package: `pkg.${label.toLowerCase()}`,
    label,
    is_system: false,
    version_name: null,
    label_source: "scrcpy",
    icon_path: null,
  };
}

async function renderReady() {
  render(<LauncherPopup />);
  await screen.findByText("WhatsApp");
  return screen.getByRole("textbox", { name: "Search apps" });
}

describe("LauncherPopup", () => {
  beforeAll(() => {
    // jsdom does not lay out, so it has no scrolling.
    Element.prototype.scrollIntoView = vi.fn();
  });

  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    launcher.target.mockResolvedValue({ kind: "ready", serial: "phone", name: "Pixel 8" });
    launcher.apps.mockResolvedValue([makeApp("WhatsApp"), makeApp("Weather")]);
    launcher.hide.mockResolvedValue(undefined);
  });

  it("opens the highlighted app and hides", async () => {
    launcher.startAppWindow.mockResolvedValue({});
    const input = await renderReady();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    await vi.waitFor(() => expect(launcher.hide).toHaveBeenCalled());
    expect(launcher.startAppWindow).toHaveBeenCalledWith("phone", "pkg.weather", "Weather");
  });

  it("keeps the popup open and shows the error when opening fails", async () => {
    launcher.startAppWindow.mockRejectedValue("Device is offline");
    const input = await renderReady();
    fireEvent.keyDown(input, { key: "Enter" });
    expect(await screen.findByText("Device is offline")).toBeInTheDocument();
    expect(launcher.hide).not.toHaveBeenCalled();
  });

  it("does nothing on Enter when no app matches", async () => {
    const input = await renderReady();
    fireEvent.change(input, { target: { value: "zzz" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(launcher.startAppWindow).not.toHaveBeenCalled();
  });

  it("says no phone is connected when the selected phone is offline", async () => {
    launcher.target.mockResolvedValue({ kind: "offline", name: "Pixel 8" });
    render(<LauncherPopup />);
    expect(await screen.findByText("No phone connected")).toBeInTheDocument();
    expect(launcher.apps).not.toHaveBeenCalled();
  });
});
