import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";

export const avdsKey = ["avds"] as const;

/** The Android Virtual Devices available locally. */
export function useAvds() {
  return useQuery({ queryKey: avdsKey, queryFn: () => ipc.avd.list() });
}

/** Boot an AVD; the emulator takes a few seconds to show up as a device. */
export function useStartAvd() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => ipc.avd.start(name),
    onSuccess: (_result, name) => {
      toast.success(`Starting ${name}…`);
      window.setTimeout(() => {
        queryClient.invalidateQueries({ queryKey: avdsKey });
        queryClient.invalidateQueries({ queryKey: ["devices"] });
      }, 5_000);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}
