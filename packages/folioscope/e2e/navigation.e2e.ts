// SPDX-License-Identifier: AGPL-3.0-or-later
// End-to-end: bakes fixtures/figma/navigation.fig, plays it in Firefox through the built player.
// Run with `pnpm --filter folioscope test:e2e` after `vp pack` and `cargo build -p folioscope-cli`.
import assert from "node:assert/strict";
import { firefox } from "playwright";
import { bakeFixtures, serve } from "./serve.ts";

const site = bakeFixtures(["navigation", "flows-and-pages", "triggers", "overlay", "scroll"]);
const server = await serve(
  site,
  (query) => `<!doctype html><meta charset="utf-8">
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/${query.get("folio")}.folio" ${query.has("controls") ? "controls" : ""} hotspots="${query.get("hotspots") ?? "on-miss"}" style="width: 375px"></folioscope-player>`,
);

const browser = await firefox.launch();
try {
  const tab = await browser.newPage({ viewport: { width: 375, height: 812 } });
  const errors: string[] = [];
  tab.on("pageerror", (e) => errors.push(e.message));
  await tab.goto(`${server.origin}/?folio=navigation`);

  // Settled on `name`: the only frame left once a transition is over.
  const settledOn = (name: string) =>
    tab.waitForFunction((name) => {
      const frames = document
        .querySelector("folioscope-player")
        ?.shadowRoot?.querySelectorAll(".stage > .frame");
      return frames?.length === 1 && frames[0]!.getAttribute("aria-label") === name;
    }, name);
  await settledOn("Home");
  console.log("✓ starts on the flow's first frame (Home)");

  await tab.locator('folioscope-player .frame button[aria-label="Open details"]').click();
  await settledOn("Details");
  console.log("✓ the hotspot navigates to Details, with its transition");

  await tab.locator('folioscope-player .frame button[aria-label="Back"]').click();
  await settledOn("Home");
  console.log("✓ the back hotspot returns to Home");

  await tab.locator('folioscope-player .frame button[aria-label="Open details"]').focus();
  await tab.keyboard.press("Enter");
  await settledOn("Details");
  await tab.keyboard.press("Escape");
  await settledOn("Home");
  console.log("✓ keyboard: Enter follows a hotspot, Escape goes back");

  await tab.goto(`${server.origin}/?folio=flows-and-pages&controls`);
  await settledOn("Sign up 1");
  const control = (label: string) =>
    tab.locator(`folioscope-player .controls [aria-label="${label}"]`);
  assert.equal(await control("Back").isDisabled(), true);
  await control("Flow").selectOption("Log in");
  await settledOn("Log in 1");
  await tab.locator('folioscope-player .frame button[aria-label="Next"]').click();
  await settledOn("Log in 2");
  assert.equal(await control("Back").isDisabled(), false);
  await control("Restart").click();
  await settledOn("Log in 1");
  await control("Page").selectOption("App");
  await settledOn("Dashboard");
  assert.equal(await control("Flow").isHidden(), true);
  console.log("✓ controls: flow and page pickers, restart, back state");

  // A click beside every hotspot flashes them, unless hotspots="never".
  const missClick = async () => {
    const stage = (await tab.locator("folioscope-player .stage").boundingBox())!;
    await tab.mouse.click(stage.x + 5, stage.y + 5);
    return tab
      .locator("folioscope-player .viewport")
      .evaluate((el) => el.classList.contains("flash"));
  };
  await tab.goto(`${server.origin}/?folio=navigation`);
  await settledOn("Home");
  assert.equal(await missClick(), true);
  await tab.goto(`${server.origin}/?folio=navigation&hotspots=never`);
  await settledOn("Home");
  assert.equal(await missClick(), false);
  console.log("✓ hotspots: a click beside them flashes them, unless hotspots=never");

  // "While hovering" and "while pressing" hold a frame, and let go of it.
  await tab.goto(`${server.origin}/?folio=triggers`);
  await settledOn("Home"); // after the splash's 1.5 s delay
  const button = (await tab
    .locator('folioscope-player .frame[aria-label="Home"] .hotspot')
    .boundingBox())!;
  await tab.mouse.move(button.x + button.width / 2, button.y + button.height / 2);
  await settledOn("Home, button hovered");
  await tab.mouse.down();
  await settledOn("Home, button pressed");
  await tab.mouse.up();
  await settledOn("Home, button hovered");
  await tab.mouse.move(button.x + button.width / 2, button.y - 40);
  await settledOn("Home");
  console.log("✓ hover and press: a frame while they last, then back");

  // Overlays open over the screen, centered on a dimmed backdrop, and close by a button or outside.
  await tab.goto(`${server.origin}/?folio=overlay`);
  await settledOn("Home");
  const menu = tab.locator('folioscope-player .overlay[data-frame] .frame[aria-label="Menu"]');
  await tab.locator('folioscope-player .frame button[aria-label="Open menu"]').click();
  await menu.waitFor();
  await tab.waitForFunction(() =>
    document
      .querySelector("folioscope-player")!
      .shadowRoot!.getAnimations()
      .every((a) => a.playState === "finished"),
  );
  const box = (await menu.boundingBox())!;
  const stage = (await tab.locator("folioscope-player .stage").boundingBox())!;
  const centered = (a: number, b: number) => Math.abs(a - b) < 1;
  assert.ok(centered(box.x - stage.x, stage.x + stage.width - (box.x + box.width)));
  assert.ok(centered(box.y - stage.y, stage.y + stage.height - (box.y + box.height)));
  assert.equal(
    await tab
      .locator("folioscope-player .backdrop")
      .evaluate((el) => getComputedStyle(el).backgroundColor),
    "rgba(0, 0, 0, 0.4)",
  );
  await tab.locator('folioscope-player .overlay button[aria-label="Close"]').click();
  await menu.waitFor({ state: "detached" });
  await tab.locator('folioscope-player .frame button[aria-label="Open menu"]').click();
  await menu.waitFor();
  await tab.mouse.click(stage.x + 5, stage.y + 5);
  await menu.waitFor({ state: "detached" });
  await settledOn("Home");
  console.log("✓ overlays: centered on a backdrop, closed by a button or a click outside");

  // A scrolling frame: the content moves under fixed layers, and "scroll to" brings a card back.
  await tab.goto(`${server.origin}/?folio=scroll`);
  await settledOn("Feed");
  const scroller = tab.locator("folioscope-player .frame .scroll");
  const scrollTop = () => scroller.evaluate((el) => el.scrollTop);
  const view = (await scroller.boundingBox())!;
  await tab.mouse.move(view.x + view.width / 2, view.y + view.height / 2);
  await tab.mouse.wheel(0, 600);
  await tab.waitForFunction(
    () =>
      document.querySelector("folioscope-player")!.shadowRoot!.querySelector(".scroll")!.scrollTop >
      0,
  );
  const layers = await tab
    .locator("folioscope-player .frame > img")
    .evaluateAll((imgs) => imgs.map((img) => img.getBoundingClientRect().top));
  assert.equal(layers.length, 2); // the background and the fixed layer stay in place
  await tab.locator('folioscope-player .content button[aria-label="Back to top"]').click();
  await tab.waitForFunction(
    () =>
      Math.abs(
        document.querySelector("folioscope-player")!.shadowRoot!.querySelector(".scroll")!
          .scrollTop - 112,
      ) < 1,
  );
  assert.ok(Math.abs((await scrollTop()) - 112) < 1);
  console.log("✓ scroll: the content scrolls under fixed layers, scroll to reaches its target");

  // Without a poster, the frame's baked placeholder shows while its layers load.
  const slow = await browser.newPage();
  await slow.route("**/layers/**", async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    await route.continue();
  });
  await slow.goto(`${server.origin}/?folio=navigation`);
  const placeholder = slow.locator("folioscope-player .poster");
  await placeholder.waitFor();
  assert.match((await placeholder.getAttribute("src")) ?? "", /^data:image\/png;base64,/);
  await slow.locator('folioscope-player .frame[aria-label="Home"]').waitFor();
  await placeholder.waitFor({ state: "detached" });
  console.log("✓ placeholder: a blurred preview until the first frame is decoded");

  assert.deepEqual(errors, []);
} finally {
  await browser.close();
  server.close();
}
