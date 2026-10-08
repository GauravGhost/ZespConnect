import { ArrowDownToLine, ArrowUpFromLine, ClipboardCopy, Info, Loader2 } from "lucide-react";
import { useState } from "react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useClipboardRead, useClipboardSend } from "../hooks";

export function ClipboardView({ serial }: { serial: string }) {
  const [text, setText] = useState("");
  const [received, setReceived] = useState<string | null>(null);
  const send = useClipboardSend(serial);
  const read = useClipboardRead(serial);

  function handleRead() {
    read.mutate(undefined, { onSuccess: (value) => setReceived(value) });
  }

  return (
    <div className="mx-auto w-full max-w-2xl space-y-4 overflow-auto p-5">
      <Alert>
        <Info />
        <AlertTitle>AdbClipboard helper</AlertTitle>
        <AlertDescription>
          Reading from the device needs the free AdbClipboard app installed, with its floating
          window tapped once to publish the clipboard. Sending to the device works with the app
          installed.
        </AlertDescription>
      </Alert>

      <div className="space-y-2">
        <label htmlFor="clipboard-text" className="text-sm font-medium">
          Send to device
        </label>
        <textarea
          id="clipboard-text"
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder="Type or paste text to put on the device clipboard"
          rows={5}
          spellCheck={false}
          className="focus-visible:bg-muted focus-visible:ring-ring min-h-24 w-full resize-y rounded-2xl border border-transparent bg-muted/40 px-4 py-3 font-mono text-sm transition-[color,box-shadow,background-color] outline-none focus-visible:ring-2"
        />
        <div className="flex justify-end">
          <Button
            size="sm"
            disabled={!text.trim() || send.isPending}
            onClick={() => send.mutate(text)}
          >
            {send.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <ArrowUpFromLine className="size-4" />
            )}
            Send to device
          </Button>
        </div>
      </div>

      <div className="space-y-2 border-t pt-4">
        <div className="flex items-center justify-between gap-2">
          <span className="text-sm font-medium">Read from device</span>
          <Button variant="outline" size="sm" disabled={read.isPending} onClick={handleRead}>
            {read.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <ArrowDownToLine className="size-4" />
            )}
            Read from device
          </Button>
        </div>
        <pre className="bg-muted/40 max-h-64 min-h-24 overflow-auto rounded-2xl p-3 font-mono text-sm whitespace-pre-wrap">
          {read.isPending
            ? "Reading…"
            : (received ??
              "Nothing read yet. The device clipboard will appear here and be copied to your computer.")}
        </pre>
        {received ? (
          <p className="text-muted-foreground inline-flex items-center gap-1.5 text-xs">
            <ClipboardCopy className="size-3.5" />
            Also placed on your computer's clipboard.
          </p>
        ) : null}
      </div>
    </div>
  );
}
