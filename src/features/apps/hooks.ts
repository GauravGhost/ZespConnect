import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { AppListResult } from "@/types";

const APK_INSTALL_TOAST = "apk-install";

export interface AppVars {
  serial: string;
  includeSystem: boolean;
}

export const appsKey = (serial: string, includeSystem: boolean) =>
  ["apps", serial, includeSystem] as const;

/** Installed apps for a device: cache-painted, refreshed on demand. */
export function useAppList(serial: string | null, includeSystem: boolean) {
  return useQuery({
    queryKey: ["apps", serial, includeSystem],
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
      queryClient.invalidateQueries({ queryKey: ["apps", vars.serial] });
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

export function useInstallApk() {
  const queryClient = useQueryClient();

  // Mirror the backend's `install:progress` lines into a single, updating toast.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    events
      .installProgress((event) => {
        const { message, progress } = event.payload;
        if (progress >= 100) {
          toast.success(message, { id: APK_INSTALL_TOAST });
        } else {
          toast.loading(message, { id: APK_INSTALL_TOAST });
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* backend not ready yet */
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return useMutation({
    mutationFn: ({ serial, path }: { serial: string; path: string }) =>
      ipc.apps.installApk(serial, path),
    onSuccess: (_result, vars) => {
      queryClient.invalidateQueries({ queryKey: ["apps", vars.serial] });
    },
    onError: (error) => toast.error(errorMessage(error), { id: APK_INSTALL_TOAST }),
  });
}
