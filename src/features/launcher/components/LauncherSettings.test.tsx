import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { LauncherStatus } from "@/types";
import { LauncherSettings } from "./LauncherSettings";

const launcher = vi.hoisted(() => ({ status: vi.fn(), save: vi.fn(), toastError: vi.fn() }));

vi.mock("@/lib/ipc", () => ({
  errorMessage: (error: unknown) => String(error),
  ipc: { launcher: { status: launcher.status, save: launcher.save } },
}));

vi.mock("sonner", () => ({ toast: { error: launcher.toastError } }));

const OFF: LauncherStatus = {
  enabled: false,
  shortcut: "CommandOrControl+Shift+Space",
  registered: false,
};

function renderSettings() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <LauncherSettings />
    </QueryClientProvider>,
  );
}

describe("LauncherSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    launcher.status.mockResolvedValue(OFF);
  });

  it("turns the launcher on with the current hotkey", async () => {
    launcher.save.mockResolvedValue({ ...OFF, enabled: true, registered: true });
    renderSettings();
    fireEvent.click(await screen.findByRole("switch"));
    await vi.waitFor(() =>
      expect(launcher.save).toHaveBeenCalledWith({
        enabled: true,
        shortcut: "CommandOrControl+Shift+Space",
      }),
    );
  });

  it("records a new hotkey from the next valid key press", async () => {
    launcher.save.mockResolvedValue({ ...OFF, shortcut: "CommandOrControl+Alt+K" });
    renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Ctrl+Shift+Space" }));
    expect(screen.getByRole("button", { name: "Press keys" })).toBeInTheDocument();
    fireEvent.keyDown(window, { code: "KeyK" });
    fireEvent.keyDown(window, { code: "KeyK", ctrlKey: true, altKey: true });
    await vi.waitFor(() =>
      expect(launcher.save).toHaveBeenCalledWith({
        enabled: false,
        shortcut: "CommandOrControl+Alt+K",
      }),
    );
    expect(launcher.save).toHaveBeenCalledTimes(1);
  });

  it("cancels recording on Escape", async () => {
    renderSettings();
    fireEvent.click(await screen.findByRole("button", { name: "Ctrl+Shift+Space" }));
    fireEvent.keyDown(window, { code: "Escape", key: "Escape" });
    expect(await screen.findByRole("button", { name: "Ctrl+Shift+Space" })).toBeInTheDocument();
    expect(launcher.save).not.toHaveBeenCalled();
  });

  it("says when another app holds the hotkey", async () => {
    launcher.status.mockResolvedValue({ ...OFF, enabled: true, registered: false });
    renderSettings();
    expect(await screen.findByText("Hotkey in use by another app")).toBeInTheDocument();
  });

  it("toasts a refused save", async () => {
    launcher.save.mockRejectedValue("Ctrl+Shift+Space is in use by another app");
    renderSettings();
    fireEvent.click(await screen.findByRole("switch"));
    await vi.waitFor(() =>
      expect(launcher.toastError).toHaveBeenCalledWith("Ctrl+Shift+Space is in use by another app"),
    );
  });
});
