import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import { pickToolExecutable } from "@/lib/toolDialogs";
import type { ToolKind, ToolStatus } from "@/types";

import { devicesKey } from "./useDevices";
import { useInstallLog } from "./useInstallLog";

export const toolsKey = ["tools"] as const;

/** Shared tool status, polled so the UI reflects installs and re-scans. */
export function useToolsStatus() {
  return useQuery({
    queryKey: toolsKey,
    queryFn: () => ipc.tools.status(),
    refetchInterval: 5_000,
  });
}

/** Set (or clear, with `path: null`) a tool's explicit binary path. */
export function useSetToolPath() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ tool, path }: { tool: ToolKind; path: string | null }) =>
      ipc.tools.setPath(tool, path),
    onSuccess: (status: ToolStatus) => {
      queryClient.setQueryData(toolsKey, status);
    },
  });
}

/** Install or update a tool by downloading it from upstream (user-triggered). */
export function useInstallTool() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (tool: ToolKind) => ipc.tools.install(tool),
    onSuccess: (status: ToolStatus) => {
      queryClient.setQueryData(toolsKey, status);
      queryClient.invalidateQueries({ queryKey: devicesKey });
    },
  });
}

/** Compare installed tools against the latest upstream releases. */
export function useCheckUpdates() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.tools.checkUpdates(),
    onSuccess: (status: ToolStatus) => {
      queryClient.setQueryData(toolsKey, status);
    },
  });
}

/** Locate, clear and install actions shared by the tool setup screens, with the installer log. */
export function useToolActions() {
  const queryClient = useQueryClient();
  const setPath = useSetToolPath();
  const install = useInstallTool();
  const log = useInstallLog();

  async function locate(kind: ToolKind) {
    const currentPath = queryClient.getQueryData<ToolStatus>(toolsKey)?.[kind].path ?? null;
    const selected = await pickToolExecutable(kind, currentPath);
    if (selected) setPath.mutate({ tool: kind, path: selected });
  }

  function clear(kind: ToolKind) {
    setPath.mutate({ tool: kind, path: null });
  }

  function selectManaged(kind: ToolKind, path: string) {
    setPath.mutate({ tool: kind, path });
  }

  function runInstall(kind: ToolKind, onSuccess?: (status: ToolStatus) => void) {
    log.reset();
    install.mutate(kind, { onError: (error) => toast.error(errorMessage(error)), onSuccess });
  }

  return { log, locate, clear, selectManaged, install: runInstall, installing: install.isPending };
}
