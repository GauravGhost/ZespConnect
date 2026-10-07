import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import type { ToolStatus } from "@/types";

export const toolsKey = ["tools"] as const;

export function useToolsStatus() {
  return useQuery({
    queryKey: toolsKey,
    queryFn: () => ipc.tools.status(),
    refetchInterval: 5_000,
  });
}

export function useSetToolPath() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ tool, path }: { tool: "adb" | "scrcpy"; path: string | null }) =>
      ipc.tools.setPath(tool, path),
    onSuccess: (status: ToolStatus) => {
      queryClient.setQueryData(toolsKey, status);
    },
  });
}
