import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { chooseApk } from "@/lib/apkPicker";
import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";

import { useBackendEvents } from "./useBackendEvents";

const APK_INSTALL_TOAST = "apk-install";

/** The app list query key; without `includeSystem` it is the prefix matching both lists. */
export const appsKey = (serial: string, includeSystem?: boolean) =>
  includeSystem === undefined
    ? (["apps", serial] as const)
    : (["apps", serial, includeSystem] as const);

/** Pick an APK and install it, mirroring the backend's `install:progress` lines into one toast. */
export function useInstallApk() {
  const queryClient = useQueryClient();

  useBackendEvents((track) => {
    track(
      events.installProgress((event) => {
        const { message, progress } = event.payload;
        if (progress >= 100) {
          toast.success(message, { id: APK_INSTALL_TOAST });
        } else {
          toast.loading(message, { id: APK_INSTALL_TOAST });
        }
      }),
    );
  }, []);

  const install = useMutation({
    mutationFn: ({ serial, path }: { serial: string; path: string }) =>
      ipc.apps.installApk(serial, path),
    onSuccess: (_result, vars) => {
      queryClient.invalidateQueries({ queryKey: appsKey(vars.serial) });
    },
    onError: (error) => toast.error(errorMessage(error), { id: APK_INSTALL_TOAST }),
  });

  async function pickAndInstall(serial: string) {
    const path = await chooseApk();
    if (path) install.mutate({ serial, path });
  }

  return { pickAndInstall, isPending: install.isPending };
}
