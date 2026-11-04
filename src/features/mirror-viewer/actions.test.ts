import { describe, expect, it } from "vitest";

import { menuActions, offersBack } from "./actions";

describe("menuActions", () => {
  it("offers panels and rotation only on the main display", () => {
    expect(menuActions({ kind: "mirror", cameraControls: false })).toContain("rotate");
    expect(menuActions({ kind: "screen-app", cameraControls: false })).toContain("notifications");
    expect(menuActions({ kind: "app", cameraControls: false })).toEqual([]);
  });

  it("offers zoom only to a camera session that supports it", () => {
    expect(menuActions({ kind: "camera", cameraControls: true })).toEqual(["zoom-in", "zoom-out"]);
    expect(menuActions({ kind: "camera", cameraControls: false })).toEqual([]);
  });

  it("leaves desktop mode without a menu", () => {
    expect(menuActions({ kind: "desktop", cameraControls: false })).toEqual([]);
  });

  it("keeps Back out of the bottom bar", () => {
    expect(menuActions({ kind: "mirror", cameraControls: false })).not.toContain("back");
  });
});

describe("offersBack", () => {
  it("offers Back only where there is an app to go back in", () => {
    expect(offersBack("mirror")).toBe(true);
    expect(offersBack("screen-app")).toBe(true);
    expect(offersBack("app")).toBe(true);
    expect(offersBack("camera")).toBe(false);
    expect(offersBack("desktop")).toBe(false);
  });
});
