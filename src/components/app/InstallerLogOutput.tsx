import { cn } from "@/lib/utils";

/** The streaming installer log, with a placeholder until output arrives. */
export function InstallerLogOutput({ lines, className }: { lines: string[]; className?: string }) {
  return (
    <pre
      className={cn(
        "bg-muted/50 overflow-auto rounded-md p-3 font-mono text-xs whitespace-pre-wrap",
        className,
      )}
    >
      {lines.length > 0 ? lines.join("\n") : "Waiting for output…"}
    </pre>
  );
}
