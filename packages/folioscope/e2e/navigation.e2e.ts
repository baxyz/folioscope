// SPDX-License-Identifier: AGPL-3.0-or-later
// End-to-end: bakes fixtures/figma/navigation.fig, plays it in Firefox through the built player.
// Run with `pnpm --filter folioscope test:e2e` after `vp pack` and `cargo build -p folioscope-cli`.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { firefox } from "playwright";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const site = mkdtempSync(join(tmpdir(), "folioscope-e2e-"));
const cli = process.env.FOLIOSCOPE_CLI ?? join(repo, "target/debug/folioscope");
for (const name of ["navigation", "flows-and-pages"]) {
  execFileSync(
    cli,
    ["bake", join(repo, `fixtures/figma/${name}.fig`), join(site, `${name}.folio`)],
    {
      stdio: "inherit",
    },
  );
}

const page = (query: URLSearchParams) => `<!doctype html><meta charset="utf-8">
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/${query.get("folio")}.folio" ${query.has("controls") ? "controls" : ""} style="width: 375px"></folioscope-player>`;
const types: Record<string, string> = {
  ".png": "image/png",
  ".json": "application/json",
  ".mjs": "text/javascript",
};
const server = createServer((req, res) => {
  const url = new URL(req.url!, "http://x");
  const path = normalize(decodeURIComponent(url.pathname));
  try {
    const body =
      path === "/"
        ? page(url.searchParams)
        : path === "/player.mjs"
          ? readFileSync(join(repo, "packages/folioscope/dist/index.mjs"))
          : readFileSync(join(site, path));
    res.writeHead(200, {
      "content-type":
        path === "/" ? "text/html" : (types[extname(path)] ?? "application/octet-stream"),
    });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise<void>((resolve) => server.listen(0, resolve));
const port = (server.address() as { port: number }).port;

const browser = await firefox.launch();
try {
  const tab = await browser.newPage({ viewport: { width: 375, height: 812 } });
  const errors: string[] = [];
  tab.on("pageerror", (e) => errors.push(e.message));
  await tab.goto(`http://localhost:${port}/?folio=navigation`);

  // Settled on `name`: the only frame left once a transition is over.
  const settledOn = (name: string) =>
    tab.waitForFunction((name) => {
      const frames = document
        .querySelector("folioscope-player")
        ?.shadowRoot?.querySelectorAll(".frame");
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

  await tab.goto(`http://localhost:${port}/?folio=flows-and-pages&controls`);
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

  // Without a poster, the frame's baked placeholder shows while its layers load.
  const slow = await browser.newPage();
  await slow.route("**/layers/**", async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    await route.continue();
  });
  await slow.goto(`http://localhost:${port}/?folio=navigation`);
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
