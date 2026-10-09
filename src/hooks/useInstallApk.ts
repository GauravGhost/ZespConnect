import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";

import { useBackendEvents } from "./useBackendEvents";

const APK_INSTALL_TOAST = "apk-install";

/** Install an APK, mirroring the backend's `install:progress` lines into one toast. */
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

  return useMutation({
    mutationFn: ({ serial, path }: { serial: string; path: string }) =>
      ipc.apps.installApk(serial, path),
    onSuccess: (_result, vars) => {
      queryClient.invalidateQueries({ queryKey: ["apps", vars.serial] });
    },
    onError: (error) => toast.error(errorMessage(error), { id: APK_INSTALL_TOAST }),
  });
}
