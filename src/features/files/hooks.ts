import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";

import { baseName, joinPath } from "./path";

export const filesKey = (serial: string, path: string) => ["files", serial, path] as const;

/** The entries in a device directory. */
export function useDirectory(serial: string | null, path: string) {
  return useQuery({
    queryKey: filesKey(serial ?? "", path),
    queryFn: () => ipc.files.listDir(serial as string, path),
    enabled: Boolean(serial) && path.length > 0,
  });
}

/** Copy a device file to a chosen host path. */
export function usePullFile(serial: string | null) {
  return useMutation({
    mutationFn: ({ remote, local }: { remote: string; local: string }) =>
      ipc.files.pull(serial as string, remote, local),
    onSuccess: (_result, vars) => toast.success(`Saved ${baseName(vars.local)}`),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Copy host files into the directory currently shown. */
export function usePushFiles(serial: string | null, path: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (locals: string[]) => {
      for (const local of locals) {
        await ipc.files.push(serial as string, local, joinPath(path, baseName(local)));
      }
    },
    onSuccess: (_result, locals) => {
      toast.success(`Uploaded ${locals.length} file${locals.length === 1 ? "" : "s"}`);
      queryClient.invalidateQueries({ queryKey: filesKey(serial ?? "", path) });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Create a folder inside the directory currently shown. */
export function useCreateFolder(serial: string | null, path: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => ipc.files.mkdir(serial as string, joinPath(path, name)),
    onSuccess: (_result, name) => {
      toast.success(`Created ${name}`);
      queryClient.invalidateQueries({ queryKey: filesKey(serial ?? "", path) });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Delete a device file or directory. */
export function useDeleteEntry(serial: string | null, path: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (target: string) => ipc.files.delete(serial as string, target),
    onSuccess: () => {
      toast.success("Deleted");
      queryClient.invalidateQueries({ queryKey: filesKey(serial ?? "", path) });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });
}
