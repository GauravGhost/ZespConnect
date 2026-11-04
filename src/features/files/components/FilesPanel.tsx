import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  ChevronRight,
  Download,
  ExternalLink,
  Folder,
  FolderPlus,
  HardDrive,
  Home,
  Loader2,
  MoreVertical,
  RefreshCw,
  Trash2,
  Upload,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { EmptyState, NoDeviceState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useActiveDevice } from "@/hooks/useDevices";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { FileEntry } from "@/types";
import {
  useCreateFolder,
  useDeleteEntry,
  useDirectory,
  useOpenOnComputer,
  usePullFile,
  usePushFiles,
} from "../hooks";
import { breadcrumbsWithin, parentWithin } from "../path";
import { previewKind } from "../preview";
import { FileEntryIcon } from "./FileEntryIcon";
import { FilePreviewDialog } from "./FilePreviewDialog";

const ROOT = "/sdcard";

export function FilesPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;

  const [path, setPath] = useState(ROOT);
  const [folderOpen, setFolderOpen] = useState(false);
  const [folderName, setFolderName] = useState("");
  const [confirm, setConfirm] = useState<FileEntry | null>(null);
  const [previewing, setPreviewing] = useState<FileEntry | null>(null);

  useEffect(() => setPath(ROOT), [serial]);

  const {
    data: entries = [],
    isLoading,
    isError,
    error,
    isFetching,
    refetch,
  } = useDirectory(serial, path);
  const previewable = useMemo(
    () => entries.filter((entry) => !entry.is_directory && previewKind(entry.name, entry.size)),
    [entries],
  );

  const pull = usePullFile(serial);
  const openFile = useOpenOnComputer(serial);
  const push = usePushFiles(serial, path);
  const createFolder = useCreateFolder(serial, path);
  const remove = useDeleteEntry(serial, path);

  if (!device) {
    return (
      <NoDeviceState
        icon={HardDrive}
        description="Connect an Android device to browse and transfer its files."
      />
    );
  }

  const crumbs = breadcrumbsWithin(path, ROOT);
  const busy = pull.isPending || push.isPending || createFolder.isPending || remove.isPending;

  function openEntry(entry: FileEntry) {
    if (entry.is_directory) setPath(entry.path);
    else if (previewKind(entry.name, entry.size)) setPreviewing(entry);
    else openFile.mutate(entry.path);
  }

  async function download(entry: FileEntry) {
    const destination = await save({ defaultPath: entry.name, title: "Save file" });
    if (typeof destination === "string") pull.mutate({ remote: entry.path, local: destination });
  }

  async function upload() {
    const selected = await open({
      multiple: true,
      directory: false,
      title: "Select files to upload",
    });
    if (selected === null) return;
    const locals = Array.isArray(selected) ? selected : [selected];
    if (locals.length > 0) push.mutate(locals);
  }

  function submitFolder(event: React.FormEvent) {
    event.preventDefault();
    const name = folderName.trim();
    if (!name) return;
    createFolder.mutate(name, {
      onSuccess: () => {
        setFolderName("");
        setFolderOpen(false);
      },
    });
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b px-4 py-2.5">
        <div className="flex items-center gap-2">
          <nav
            className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto text-sm [scrollbar-width:none]"
            aria-label="Breadcrumb"
          >
            <button
              type="button"
              aria-label="Go to device storage"
              title="Device storage"
              className={cn(
                "inline-flex shrink-0 items-center gap-1.5 rounded-full px-2.5 py-1.5",
                path === ROOT
                  ? "bg-primary-container text-on-primary-container"
                  : "bg-muted/60 text-muted-foreground hover:text-foreground",
              )}
              onClick={() => setPath(ROOT)}
            >
              <Home className="size-3.5" />
              <span className="font-mono text-xs">{ROOT}</span>
            </button>
            {crumbs.map((crumb) => (
              <span key={crumb.path} className="flex shrink-0 items-center gap-1">
                <ChevronRight className="text-muted-foreground size-3.5" />
                <button
                  type="button"
                  onClick={() => setPath(crumb.path)}
                  className={cn(
                    "rounded-full px-2.5 py-1.5 font-mono text-xs",
                    crumb.path === path
                      ? "bg-primary-container text-on-primary-container"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {crumb.label}
                </button>
              </span>
            ))}
          </nav>
          <div className="flex shrink-0 items-center gap-1.5">
            <Button
              variant="ghost"
              size="icon"
              className="size-9"
              aria-label="Refresh folder"
              title="Refresh folder"
              disabled={isFetching}
              onClick={() => refetch()}
            >
              <RefreshCw className={cn("size-4", isFetching && "animate-spin")} />
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-9 gap-2"
              disabled={busy}
              onClick={() => {
                setFolderName("");
                setFolderOpen(true);
              }}
            >
              <FolderPlus className="size-4" />
              New folder
            </Button>
            <Button size="sm" className="h-9 gap-2" disabled={busy} onClick={upload}>
              <Upload className="size-4" />
              Upload
            </Button>
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-3 py-1.5">
        {isLoading ? (
          <div className="flex h-40 items-center justify-center">
            <Loader2 className="text-muted-foreground size-5 animate-spin" />
          </div>
        ) : isError ? (
          <Alert variant="destructive" className="my-6">
            <AlertTriangle />
            <AlertTitle>Could not open this folder</AlertTitle>
            <AlertDescription>{errorMessage(error)}</AlertDescription>
          </Alert>
        ) : entries.length === 0 ? (
          <div className="py-10">
            <EmptyState
              icon={Folder}
              title="This folder is empty"
              description="Upload a file or go back to another folder."
            />
          </div>
        ) : (
          <ul className="flex flex-col gap-0.5">
            {path !== ROOT ? (
              <li>
                <button
                  type="button"
                  className="hover:bg-accent/50 flex h-10 w-full items-center gap-3 rounded-2xl px-3 text-left text-sm"
                  onClick={() => setPath(parentWithin(path, ROOT))}
                >
                  <Folder className="text-muted-foreground size-4 shrink-0" />
                  <span className="font-medium">..</span>
                </button>
              </li>
            ) : null}
            {entries.map((entry) => {
              const opening = openFile.isPending && openFile.variables === entry.path;
              return (
                <li
                  key={entry.path}
                  className="hover:bg-accent/40 flex items-center gap-3 rounded-2xl py-1 pr-1 pl-3"
                >
                  <button
                    type="button"
                    disabled={opening}
                    onClick={() => openEntry(entry)}
                    className="flex min-w-0 flex-1 items-center gap-3 text-left disabled:cursor-progress"
                  >
                    {opening ? (
                      <Loader2 className="text-muted-foreground size-4 shrink-0 animate-spin" />
                    ) : (
                      <FileEntryIcon entry={entry} />
                    )}
                    <span className="min-w-0 flex-1 truncate text-sm">{entry.name}</span>
                    <span
                      className="text-muted-foreground hidden shrink-0 font-mono text-[11px] md:inline"
                      title={entry.permissions}
                    >
                      {entry.date}
                    </span>
                    <span className="text-muted-foreground w-20 shrink-0 text-right text-xs tabular-nums">
                      {entry.is_directory ? "-" : entry.formatted_size}
                    </span>
                  </button>

                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-8 shrink-0"
                        aria-label={`Actions for ${entry.name}`}
                        disabled={busy}
                      >
                        <MoreVertical className="size-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      {entry.is_directory ? null : (
                        <DropdownMenuItem disabled={opening} onSelect={() => openEntry(entry)}>
                          <ExternalLink />
                          Open
                        </DropdownMenuItem>
                      )}
                      <DropdownMenuItem
                        disabled={entry.is_directory}
                        onSelect={() => {
                          if (!entry.is_directory) void download(entry);
                        }}
                      >
                        <Download />
                        Download
                      </DropdownMenuItem>
                      <DropdownMenuItem variant="destructive" onSelect={() => setConfirm(entry)}>
                        <Trash2 />
                        Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <FilePreviewDialog
        serial={device.serial}
        files={previewable}
        current={previewing}
        onShow={setPreviewing}
        onOpenInDefaultApp={(entry) => openFile.mutate(entry.path)}
        onDownload={(entry) => void download(entry)}
      />

      <Dialog
        open={folderOpen}
        onOpenChange={(next) => {
          setFolderOpen(next);
          if (!next) setFolderName("");
        }}
      >
        <DialogContent>
          <form onSubmit={submitFolder} className="space-y-4">
            <DialogHeader>
              <DialogTitle>New folder</DialogTitle>
              <DialogDescription>Create a folder in {path}.</DialogDescription>
            </DialogHeader>
            <div className="space-y-1.5">
              <Label htmlFor="folder-name" className="text-muted-foreground text-xs">
                Folder name
              </Label>
              <Input
                id="folder-name"
                value={folderName}
                onChange={(event) => setFolderName(event.target.value)}
                placeholder="New folder"
                autoComplete="off"
                spellCheck={false}
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setFolderOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={!folderName.trim() || createFolder.isPending}>
                {createFolder.isPending ? <Loader2 className="animate-spin" /> : <FolderPlus />}
                Create
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={confirm !== null} onOpenChange={(next) => !next && setConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete {confirm?.name}?</DialogTitle>
            <DialogDescription>
              This permanently removes the{" "}
              {confirm?.is_directory ? "folder and everything in it" : "file"} from the device. It
              cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirm(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={remove.isPending}
              onClick={() => {
                if (confirm) remove.mutate(confirm.path, { onSettled: () => setConfirm(null) });
              }}
            >
              {remove.isPending ? <Loader2 className="animate-spin" /> : <Trash2 />}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
