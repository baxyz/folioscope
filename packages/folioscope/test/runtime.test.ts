// SPDX-License-Identifier: AGPL-3.0-or-later
import { describe, expect, test } from "vite-plus/test";
import type { FolioFrame, FolioPage, Transition } from "../src/folio.ts";
import { Runtime } from "../src/runtime.ts";

const dissolve: Transition = {
  kind: { type: "dissolve" },
  durationMs: 300,
  easing: { type: "easeOut" },
};
const box = { x: 0, y: 0, width: 375, height: 812 };

function frame(id: string, links: Record<string, string> = {}): FolioFrame {
  return {
    id,
    name: id,
    bounds: box,
    layers: [],
    hotspots: Object.entries(links).map(([name, to]) => ({
      node: `${id}/${name}`,
      name,
      bounds: box,
      interactions: [
        { trigger: { type: "click" }, action: { type: "navigate", to, transition: dissolve } },
      ],
    })),
  };
}

const page: FolioPage = {
  id: "0:1",
  name: "Prototype",
  flows: [
    { name: "Main", start: "home" },
    { name: "Settings", start: "settings" },
  ],
  frames: [
    frame("home", { open: "details" }),
    frame("details", { next: "settings" }),
    frame("settings"),
  ],
};

describe("Runtime", () => {
  test("starts on the first flow, a named flow, or a flow index", () => {
    expect(new Runtime(page).frame.id).toBe("home");
    expect(new Runtime(page, "Settings").frame.id).toBe("settings");
    expect(new Runtime(page, "1").frame.id).toBe("settings");
    expect(new Runtime({ ...page, flows: [] }).frame.id).toBe("home");
  });

  test("navigates, then goes back with the reversed transition", () => {
    const runtime = new Runtime(page);
    const forward = runtime.run({ type: "navigate", to: "details", transition: dissolve });
    expect(forward?.frame.id).toBe("details");
    expect(forward?.transition).toEqual(dissolve);
    expect(runtime.canGoBack).toBe(true);

    const back = runtime.run({ type: "back" });
    expect(back).toMatchObject({ frame: { id: "home" }, transition: dissolve, reverse: true });
    expect(runtime.canGoBack).toBe(false);
    expect(runtime.run({ type: "back" })).toBeUndefined();
  });

  test("ignores links to unknown frames or to the current one", () => {
    const runtime = new Runtime(page);
    expect(runtime.goTo("nowhere")).toBeUndefined();
    expect(runtime.goTo("home")).toBeUndefined();
  });

  test("restart clears the history", () => {
    const runtime = new Runtime(page);
    runtime.goTo("details");
    runtime.goTo("settings");
    expect(runtime.restart().frame.id).toBe("home");
    expect(runtime.canGoBack).toBe(false);
  });

  test("fails on a page with nothing to show", () => {
    expect(() => new Runtime({ ...page, flows: [], frames: [] })).toThrow(/no frame/);
  });
});
