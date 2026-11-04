import {
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  Download,
  ExternalLink,
  Loader2,
  Music,
  X,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { FileEntry } from "@/types";
import { useTextPreview } from "../hooks";
import { deviceFileUrl, type PreviewKind, previewKind } from "../preview";
import { FileEntryIcon } from "./FileEntryIcon";

interface FilePreviewDialogProps {
  serial: string;
  /** The previewable files of the current folder, in list order, for previous/next. */
  files: FileEntry[];
  /** The file shown, or null when closed. */
  current: FileEntry | null;
  onShow: (entry: FileEntry | null) => void;
  onOpenInDefaultApp: (entry: FileEntry) => void;
  onDownload: (entry: FileEntry) => void;
}

export function FilePreviewDialog({
  serial,
  files,
  current,
  onShow,
  onOpenInDefaultApp,
  onDownload,
}: FilePreviewDialogProps) {
  const index = current ? files.findIndex((file) => file.path === current.path) : -1;
  const previous = index > 0 ? files[index - 1] : null;
  const next = index >= 0 && index < files.length - 1 ? files[index + 1] : null;

  function navigateWithArrows(event: React.KeyboardEvent) {
    // Media controls use the arrow keys to seek.
    if (event.target instanceof HTMLMediaElement) return;
    if (event.key === "ArrowLeft" && previous) onShow(previous);
    if (event.key === "ArrowRight" && next) onShow(next);
  }

  return (
    <Dialog open={current !== null} onOpenChange={(open) => !open && onShow(null)}>
      <DialogContent
        showCloseButton={false}
        className="flex h-[calc(100vh-4rem)] w-[calc(100vw-4rem)] max-w-none flex-col gap-0 overflow-hidden rounded-3xl p-0 sm:max-w-none"
        onKeyDown={navigateWithArrows}
      >
        {current ? (
          <>
            <header className="flex shrink-0 items-center gap-1 border-b py-2.5 pr-2.5 pl-5">
              <FileEntryIcon entry={current} />
              <div className="ml-2 min-w-0 flex-1">
                <DialogTitle className="truncate text-sm leading-tight">{current.name}</DialogTitle>
                <DialogDescription className="text-xs">
                  {current.formatted_size}
                  {files.length > 1 ? ` - ${index + 1} of ${files.length}` : null}
                </DialogDescription>
              </div>

              <ToolbarButton
                label="Previous file"
                disabled={!previous}
                onClick={() => previous && onShow(previous)}
              >
                <ChevronLeft />
              </ToolbarButton>
              <ToolbarButton
                label="Next file"
                disabled={!next}
                onClick={() => next && onShow(next)}
              >
                <ChevronRight />
              </ToolbarButton>
              <span className="bg-border mx-1.5 h-5 w-px" aria-hidden />
              <ToolbarButton label="Download" onClick={() => onDownload(current)}>
                <Download />
              </ToolbarButton>
              <ToolbarButton
                label="Open with default app"
                onClick={() => onOpenInDefaultApp(current)}
              >
                <ExternalLink />
              </ToolbarButton>
              <span className="bg-border mx-1.5 h-5 w-px" aria-hidden />
              <DialogClose asChild>
                <ToolbarButton label="Close">
                  <X />
                </ToolbarButton>
              </DialogClose>
            </header>

            <div className="bg-muted/30 relative flex min-h-0 flex-1 items-center justify-center p-6">
              <PreviewBody
                key={current.path}
                entry={current}
                url={deviceFileUrl(serial, current.path)}
                onOpenInDefaultApp={() => onOpenInDefaultApp(current)}
              />
            </div>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function ToolbarButton({
  label,
  className,
  ...props
}: React.ComponentProps<typeof Button> & { label: string }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      title={label}
      className={cn("size-8 shrink-0 [&_svg]:size-4", className)}
      {...props}
    />
  );
}

interface PreviewBodyProps {
  entry: FileEntry;
  url: string;
  onOpenInDefaultApp: () => void;
}

function PreviewBody({ entry, url, onOpenInDefaultApp }: PreviewBodyProps) {
  const [failed, setFailed] = useState(false);
  const kind = previewKind(entry.name, entry.size);

  if (failed || kind === null) {
    return <PreviewUnavailable onOpenInDefaultApp={onOpenInDefaultApp} />;
  }
  return <PreviewStage kind={kind} url={url} name={entry.name} onFail={() => setFailed(true)} />;
}

function PreviewStage({
  kind,
  url,
  name,
  onFail,
}: {
  kind: PreviewKind;
  url: string;
  name: string;
  onFail: () => void;
}) {
  switch (kind) {
    case "image":
      return <ImagePreview url={url} name={name} onFail={onFail} />;
    case "video":
      return (
        <video
          src={url}
          controls
          autoPlay
          onError={onFail}
          className="max-h-full max-w-full rounded-xl bg-black shadow-sm"
        />
      );
    case "audio":
      return (
        <div className="flex w-full max-w-md flex-col items-center gap-6">
          <div className="bg-muted flex size-28 items-center justify-center rounded-3xl">
            <Music className="text-muted-foreground size-12" />
          </div>
          <audio src={url} controls autoPlay onError={onFail} className="w-full" />
        </div>
      );
    case "pdf":
      return (
        <iframe
          src={url}
          title={name}
          className="size-full rounded-xl border-0 bg-white shadow-sm"
        />
      );
    case "text":
      return <TextPreview url={url} />;
  }
}

function ImagePreview({ url, name, onFail }: { url: string; name: string; onFail: () => void }) {
  const [loaded, setLoaded] = useState(false);
  return (
    <>
      {loaded ? null : <Loader2 className="text-muted-foreground absolute size-5 animate-spin" />}
      <img
        src={url}
        alt={name}
        onLoad={() => setLoaded(true)}
        onError={onFail}
        className={cn(
          "max-h-full max-w-full rounded-xl object-contain shadow-sm transition-opacity",
          !loaded && "opacity-0",
        )}
      />
    </>
  );
}

function TextPreview({ url }: { url: string }) {
  const { data, isLoading, isError, error } = useTextPreview(url);
  if (isLoading) return <Loader2 className="text-muted-foreground size-5 animate-spin" />;
  if (isError) return <p className="text-destructive text-sm">{errorMessage(error)}</p>;
  return (
    <pre className="bg-background size-full overflow-auto rounded-xl border p-5 font-mono text-xs whitespace-pre-wrap">
      {data}
    </pre>
  );
}

function PreviewUnavailable({ onOpenInDefaultApp }: { onOpenInDefaultApp: () => void }) {
  return (
    <div className="flex flex-col items-center gap-3 text-center">
      <AlertTriangle className="text-muted-foreground size-6" />
      <p className="text-muted-foreground text-sm">This file can't be previewed in the app.</p>
      <Button size="sm" variant="outline" onClick={onOpenInDefaultApp}>
        <ExternalLink />
        Open with default app
      </Button>
    </div>
  );
}
