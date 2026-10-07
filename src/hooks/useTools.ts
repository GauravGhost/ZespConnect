import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import type { ToolKind, ToolStatus } from "@/types";

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
      queryClient.invalidateQueries({ queryKey: ["devices"] });
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
