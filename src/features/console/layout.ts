export interface Bounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface LayoutNode {
  /** Position in document order, unique within one snapshot. */
  id: number;
  depth: number;
  className: string;
  resourceId: string;
  text: string;
  contentDesc: string;
  packageName: string;
  bounds: Bounds;
  /** The uiautomator boolean attributes that are true, e.g. "clickable". */
  flags: string[];
}

const FLAG_ATTRIBUTES = [
  "clickable",
  "long-clickable",
  "scrollable",
  "checkable",
  "checked",
  "focusable",
  "focused",
  "selected",
  "enabled",
  "password",
] as const;

const BOUNDS_PATTERN = /^\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]$/;

/** uiautomator writes bounds as "[left,top][right,bottom]". */
export function parseBounds(value: string): Bounds | null {
  const match = BOUNDS_PATTERN.exec(value.trim());
  if (!match) return null;
  const [left, top, right, bottom] = match.slice(1).map(Number);
  return { left, top, right, bottom };
}

/** Every node of a uiautomator dump in document order (parents before their children). */
export function parseLayout(xml: string): LayoutNode[] {
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  if (doc.querySelector("parsererror")) throw new Error("The layout dump is not valid XML.");
  const nodes: LayoutNode[] = [];

  function visit(element: Element, depth: number) {
    const bounds = parseBounds(element.getAttribute("bounds") ?? "");
    if (bounds) {
      nodes.push({
        id: nodes.length,
        depth,
        className: element.getAttribute("class") ?? "",
        resourceId: element.getAttribute("resource-id") ?? "",
        text: element.getAttribute("text") ?? "",
        contentDesc: element.getAttribute("content-desc") ?? "",
        packageName: element.getAttribute("package") ?? "",
        bounds,
        flags: FLAG_ATTRIBUTES.filter((name) => element.getAttribute(name) === "true"),
      });
    }
    for (const child of element.children) {
      if (child.tagName === "node") visit(child, depth + 1);
    }
  }

  for (const top of doc.documentElement.children) {
    if (top.tagName === "node") visit(top, 0);
  }
  return nodes;
}

function area(bounds: Bounds): number {
  return (bounds.right - bounds.left) * (bounds.bottom - bounds.top);
}

/** The smallest node under a screen point; on equal size the later node, drawn on top, wins. */
export function nodeAt(nodes: LayoutNode[], x: number, y: number): LayoutNode | null {
  let best: LayoutNode | null = null;
  for (const node of nodes) {
    const { left, top, right, bottom } = node.bounds;
    if (x < left || x >= right || y < top || y >= bottom) continue;
    if (!best || area(node.bounds) <= area(best.bounds)) best = node;
  }
  return best;
}

/** A tree row label such as "Button #ok_button "OK"". */
export function nodeLabel(node: LayoutNode): string {
  const shortClass = node.className.split(".").pop() || "node";
  const id = node.resourceId ? ` #${node.resourceId.split("/").pop()}` : "";
  const caption = node.text || node.contentDesc;
  return caption ? `${shortClass}${id} "${caption}"` : `${shortClass}${id}`;
}

export function formatBounds({ left, top, right, bottom }: Bounds): string {
  return `[${left},${top}][${right},${bottom}]`;
}
