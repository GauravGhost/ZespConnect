import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useAppStore } from "@/app/store";
import type { ToolInfo, ToolStatus } from "@/types";
import { ToolStatusBanner } from "./ToolStatusBanner";

const toolsStatus = vi.hoisted(() => ({ current: undefined as ToolStatus | undefined }));

vi.mock("@/hooks/useTools", () => ({
  useToolsStatus: () => ({ data: toolsStatus.current }),
}));

vi.mock("@/hooks/useInstallLog", () => ({
  useInstallLog: () => ({ running: false, tool: null }),
}));

function makeTool(version: string): ToolInfo {
  return { found: true, path: "/bin/tool", version, source: "path", managed_path: null };
}

function statusWithScrcpyUpdate(latest: string): ToolStatus {
  return {
    adb: makeTool("35.0"),
    scrcpy: makeTool("5.0"),
    adb_update: { latest_version: null, outdated: false, error: null },
    scrcpy_update: { latest_version: latest, outdated: true, error: null },
  };
}

describe("ToolStatusBanner", () => {
  beforeEach(() => {
    useAppStore.setState({ dismissedScrcpyUpdate: null });
  });

  it("hides the update notice once dismissed", () => {
    toolsStatus.current = statusWithScrcpyUpdate("5.0.1");
    render(<ToolStatusBanner />);

    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(screen.queryByText(/scrcpy v5.0.1 is available/)).toBeNull();
  });

  it("shows the notice again for a newer version", () => {
    useAppStore.setState({ dismissedScrcpyUpdate: "5.0.1" });
    toolsStatus.current = statusWithScrcpyUpdate("5.0.2");
    render(<ToolStatusBanner />);

    expect(screen.getByText(/scrcpy v5.0.2 is available/)).toBeTruthy();
  });

  it("does not offer dismiss when scrcpy is missing", () => {
    toolsStatus.current = {
      ...statusWithScrcpyUpdate("5.0.1"),
      scrcpy: { ...makeTool("5.0"), found: false, version: null },
    };
    render(<ToolStatusBanner />);

    expect(screen.getByText(/scrcpy is not installed/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Dismiss" })).toBeNull();
  });
});
