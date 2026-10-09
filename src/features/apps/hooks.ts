import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { appsKey } from "@/hooks/useInstallApk";
import { errorMessage, ipc } from "@/lib/ipc";
import type { AppListResult } from "@/types";

export interface AppVars {
  serial: string;
  includeSystem: boolean;
}

/** Installed apps for a device: cache-painted, refreshed on demand. */
export function useAppList(serial: string | null, includeSystem: boolean) {
  return useQuery({
    queryKey: appsKey(serial ?? "", includeSystem),
    queryFn: () => ipc.apps.list(serial as string, includeSystem, false),
    enabled: Boolean(serial),
    staleTime: 5 * 60_000,
  });
}

/** Re-read the list from the device (scrcpy/pm) and replace the cached copy. */
export function useRefreshAppList() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: AppVars) => ipc.apps.list(vars.serial, vars.includeSystem, true),
    onSuccess: (result: AppListResult, vars) => {
      queryClient.setQueryData(appsKey(vars.serial, vars.includeSystem), result);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useLaunchApp() {
  return useMutation({
    mutationFn: ({ serial, pkg }: { serial: string; pkg: string }) => ipc.apps.launch(serial, pkg),
    onSuccess: () => toast.success("App launched"),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useUninstallApp() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, pkg }: { serial: string; pkg: string }) =>
      ipc.apps.uninstall(serial, pkg),
    onSuccess: (_result, vars) => {
      toast.success(`Uninstalled ${vars.pkg}`);
      queryClient.invalidateQueries({ queryKey: appsKey(vars.serial) });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useClearAppData() {
  return useMutation({
    mutationFn: ({ serial, pkg }: { serial: string; pkg: string }) =>
      ipc.apps.clearData(serial, pkg),
    onSuccess: (_result, vars) => toast.success(`Cleared data for ${vars.pkg}`),
    onError: (error) => toast.error(errorMessage(error)),
  });
}
