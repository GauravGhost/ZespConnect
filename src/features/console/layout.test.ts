import { describe, expect, it } from "vitest";

import { nodeAt, nodeLabel, parseBounds, parseLayout } from "./layout";

const DUMP = `<?xml version='1.0' encoding='UTF-8' standalone='yes' ?>
<hierarchy rotation="0">
  <node class="android.widget.FrameLayout" resource-id="" text="" content-desc="" package="com.example" clickable="false" enabled="true" bounds="[0,0][1080,2400]">
    <node class="android.widget.Button" resource-id="com.example:id/ok" text="OK" content-desc="" package="com.example" clickable="true" enabled="true" bounds="[100,200][500,400]" />
    <node class="android.widget.ImageView" resource-id="" text="" content-desc="Logo" package="com.example" clickable="false" enabled="true" bounds="[100,200][500,400]" />
  </node>
</hierarchy>`;

describe("parseBounds", () => {
  it("reads uiautomator bounds", () => {
    expect(parseBounds("[0,63][1080,2337]")).toEqual({
      left: 0,
      top: 63,
      right: 1080,
      bottom: 2337,
    });
  });

  it("rejects anything else", () => {
    expect(parseBounds("")).toBeNull();
    expect(parseBounds("0,0,1,1")).toBeNull();
  });
});

describe("parseLayout", () => {
  it("lists nodes in document order with depth and true flags", () => {
    const nodes = parseLayout(DUMP);
    expect(nodes.map((node) => [node.id, node.depth])).toEqual([
      [0, 0],
      [1, 1],
      [2, 1],
    ]);
    expect(nodes[1].resourceId).toBe("com.example:id/ok");
    expect(nodes[1].flags).toEqual(["clickable", "enabled"]);
  });

  it("throws on broken XML", () => {
    expect(() => parseLayout("<hierarchy><node")).toThrow();
  });
});

describe("nodeAt", () => {
  const nodes = parseLayout(DUMP);

  it("picks the smallest node, preferring the later one on a tie", () => {
    expect(nodeAt(nodes, 300, 300)?.id).toBe(2);
    expect(nodeAt(nodes, 900, 900)?.id).toBe(0);
  });

  it("finds nothing outside every node", () => {
    expect(nodeAt(nodes, 2000, 100)).toBeNull();
  });
});

describe("nodeLabel", () => {
  it("shows short class, id and caption", () => {
    const nodes = parseLayout(DUMP);
    expect(nodeLabel(nodes[1])).toBe('Button #ok "OK"');
    expect(nodeLabel(nodes[2])).toBe('ImageView "Logo"');
    expect(nodeLabel(nodes[0])).toBe("FrameLayout");
  });
});
