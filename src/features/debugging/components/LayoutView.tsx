import { save } from "@tauri-apps/plugin-dialog";
import { Copy, Download, LayoutTemplate, Loader2, ScanSearch } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { fileStamp } from "@/lib/format";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { copyText } from "../copy";
import { useLayoutCapture, type LayoutSnapshot } from "../hooks";
import { formatBounds, nodeAt, nodeLabel, type Bounds, type LayoutNode } from "../layout";

const INDENT_PX = 12;

function BoundsRect({ bounds, className }: { bounds: Bounds; className: string }) {
  return (
    <rect
      x={bounds.left}
      y={bounds.top}
      width={bounds.right - bounds.left}
      height={bounds.bottom - bounds.top}
      vectorEffect="non-scaling-stroke"
      strokeWidth={2}
      className={className}
    />
  );
}

/** The screenshot drawn in device pixels, so node bounds overlay it without any scaling math. */
function ScreenPreview({
  snapshot,
  hovered,
  selected,
  onHover,
  onSelect,
}: {
  snapshot: LayoutSnapshot;
  hovered: LayoutNode | null;
  selected: LayoutNode | null;
  onHover: (node: LayoutNode | null) => void;
  onSelect: (node: LayoutNode | null) => void;
}) {
  function nodeUnder(event: React.MouseEvent<SVGSVGElement>): LayoutNode | null {
    const svg = event.currentTarget;
    const toDevice = svg.getScreenCTM()?.inverse();
    if (!toDevice) return null;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(toDevice);
    return nodeAt(snapshot.nodes, point.x, point.y);
  }

  return (
    <svg
      viewBox={`0 0 ${snapshot.width} ${snapshot.height}`}
      className="h-full w-full cursor-crosshair"
      onMouseMove={(event) => onHover(nodeUnder(event))}
      onMouseLeave={() => onHover(null)}
      onClick={(event) => {
        const node = nodeUnder(event);
        onSelect(node?.id === selected?.id ? null : node);
      }}
    >
      <image href={snapshot.imageUrl} width={snapshot.width} height={snapshot.height} />
      {hovered && hovered.id !== selected?.id ? (
        <BoundsRect bounds={hovered.bounds} className="fill-primary/10 stroke-primary" />
      ) : null}
      {selected ? (
        <BoundsRect bounds={selected.bounds} className="fill-primary/25 stroke-primary" />
      ) : null}
    </svg>
  );
}

function NodeTree({
  nodes,
  hovered,
  selected,
  onHover,
  onSelect,
}: {
  nodes: LayoutNode[];
  hovered: LayoutNode | null;
  selected: LayoutNode | null;
  onHover: (node: LayoutNode | null) => void;
  onSelect: (node: LayoutNode | null) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const rows = useRef(new Map<number, HTMLButtonElement>());

  // Vertical only: scrollIntoView would also scroll sideways to the start of a deep, wide row.
  useEffect(() => {
    const list = container.current;
    const row = selected ? rows.current.get(selected.id) : undefined;
    if (!list || !row) return;
    const bottom = row.offsetTop + row.offsetHeight;
    if (row.offsetTop < list.scrollTop) list.scrollTop = row.offsetTop;
    else if (bottom > list.scrollTop + list.clientHeight)
      list.scrollTop = bottom - list.clientHeight;
  }, [selected]);

  return (
    <div
      ref={container}
      className="relative min-h-0 flex-1 overflow-auto py-1"
      onMouseLeave={() => onHover(null)}
    >
      {nodes.map((node) => (
        <button
          key={node.id}
          type="button"
          ref={(element) => {
            if (element) rows.current.set(node.id, element);
            else rows.current.delete(node.id);
          }}
          style={{ paddingLeft: 8 + node.depth * INDENT_PX }}
          className={cn(
            "block w-max min-w-full pr-3 text-left font-mono text-xs leading-6 whitespace-nowrap",
            node.id === selected?.id
              ? "bg-primary/15 text-foreground"
              : node.id === hovered?.id
                ? "bg-accent"
                : "text-muted-foreground hover:text-foreground",
          )}
          onMouseEnter={() => onHover(node)}
          onClick={() => onSelect(node.id === selected?.id ? null : node)}
        >
          {nodeLabel(node)}
        </button>
      ))}
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="group flex items-start gap-2 py-1">
      <dt className="text-muted-foreground w-24 shrink-0 text-xs">{label}</dt>
      <dd className="min-w-0 flex-1 font-mono text-xs break-all">
        {value || <span className="text-muted-foreground">-</span>}
      </dd>
      {value ? (
        <Button
          variant="ghost"
          size="icon"
          className="size-6 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          aria-label={`Copy ${label.toLowerCase()}`}
          title={`Copy ${label.toLowerCase()}`}
          onClick={() => void copyText(value, label)}
        >
          <Copy className="size-3.5" />
        </Button>
      ) : null}
    </div>
  );
}

function NodeDetails({ node, pinned }: { node: LayoutNode | null; pinned: boolean }) {
  if (!node) {
    return (
      <p className="text-muted-foreground p-4 text-xs">
        Point at the screenshot or the tree to see a view. Click to keep it selected.
      </p>
    );
  }
  const { bounds } = node;
  return (
    <div className="p-4 pt-3">
      <p className="text-muted-foreground mb-1 text-xs font-medium">
        {pinned ? "Selected view" : "View under pointer"}
      </p>
      <dl>
        <DetailRow label="Class" value={node.className} />
        <DetailRow label="Resource id" value={node.resourceId} />
        <DetailRow label="Text" value={node.text} />
        <DetailRow label="Description" value={node.contentDesc} />
        <DetailRow label="Package" value={node.packageName} />
        <DetailRow label="Bounds" value={formatBounds(bounds)} />
        <DetailRow
          label="Size"
          value={`${bounds.right - bounds.left} x ${bounds.bottom - bounds.top}`}
        />
      </dl>
      {node.flags.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-1">
          {node.flags.map((flag) => (
            <Badge key={flag} variant="secondary" className="font-mono text-[10px]">
              {flag}
            </Badge>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function LayoutView({ serial }: { serial: string }) {
  const { snapshot, capturing, error, capture } = useLayoutCapture(serial);
  const [hovered, setHovered] = useState<LayoutNode | null>(null);
  const [selected, setSelected] = useState<LayoutNode | null>(null);

  useEffect(() => {
    setHovered(null);
    setSelected(null);
  }, [snapshot]);

  async function saveXml() {
    if (!snapshot) return;
    const path = await save({
      defaultPath: `layout-${fileStamp(new Date())}.xml`,
      filters: [{ name: "XML", extensions: ["xml"] }],
      title: "Save layout",
    });
    if (typeof path !== "string") return;
    try {
      await ipc.console.saveLog(path, snapshot.xml);
      toast.success("Layout saved");
    } catch (failure) {
      toast.error(errorMessage(failure));
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b px-4 py-2">
        <Button variant="outline" size="sm" onClick={capture} disabled={capturing}>
          {capturing ? <Loader2 className="size-3.5 animate-spin" /> : <ScanSearch />}
          {snapshot ? "Capture again" : "Capture screen"}
        </Button>
        {snapshot ? (
          <span className="text-muted-foreground text-xs">{snapshot.nodes.length} views</span>
        ) : null}
        {error ? <span className="text-destructive truncate text-xs">{error}</span> : null}
        <Button
          variant="ghost"
          size="icon"
          className="ml-auto"
          onClick={() => void saveXml()}
          disabled={!snapshot}
          aria-label="Save layout XML"
          title="Save layout XML"
        >
          <Download />
        </Button>
      </div>

      {snapshot ? (
        <div className="flex min-h-0 flex-1">
          <div className="bg-muted/40 min-w-0 flex-1 p-4">
            <ScreenPreview
              snapshot={snapshot}
              hovered={hovered}
              selected={selected}
              onHover={setHovered}
              onSelect={setSelected}
            />
          </div>
          <div className="flex w-[min(26rem,50%)] shrink-0 flex-col border-l">
            <NodeTree
              nodes={snapshot.nodes}
              hovered={hovered}
              selected={selected}
              onHover={setHovered}
              onSelect={setSelected}
            />
            {/* Fixed height so the tree above does not resize as the details change. */}
            <div className="h-[45%] shrink-0 overflow-auto border-t">
              <NodeDetails node={selected ?? hovered} pinned={selected !== null} />
            </div>
          </div>
        </div>
      ) : (
        <div className="text-muted-foreground flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
          {capturing ? (
            <Loader2 className="size-6 animate-spin" />
          ) : (
            <LayoutTemplate className="size-6" />
          )}
          <p className="max-w-sm text-sm">
            {capturing
              ? "Reading the screen layout. This takes a few seconds."
              : "Capture the phone's screen to inspect its views: bounds, ids, text and state."}
          </p>
        </div>
      )}
    </div>
  );
}
