// SPDX-License-Identifier: AGPL-3.0-or-later
// The comparison page's network. While the page runs a comparison, the files it loads (under
// /comparison/, /folio/, /examples/ and /assets/), and those its workers load (the full player's
// WebAssembly), come through here: from the network every time, as on a first visit, delayed by
// the chosen profile, and counted. Each side of the comparison gets a link of its own. Other pages
// pass untouched, and a third-party iframe's requests never reach a site's worker.

const PATHS = ["/comparison/", "/folio/", "/examples/", "/assets/"];
/** What a server compresses: counted as gzip sends it. */
const COMPRESSED = /^(text\/|application\/(javascript|json|wasm|xml)|image\/svg)/;

/**
 * The page running a comparison: { id, run, folio (the right side's path), left (the paths of
 * the other side), profile }.
 */
let owner;
/** When each side's link is free again, in ms on this worker's clock. */
const links = { left: 0, right: 0 };

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("message", (event) => {
  const source = event.source;
  if (!source || !event.data) return;
  if (event.data.type === "run") {
    const { run, folio, left, profile } = event.data;
    owner = { id: source.id, run, folio, left, profile };
    links.left = links.right = 0;
    source.postMessage({ type: "ready", run });
  } else if (event.data.type === "claim") {
    event.waitUntil(self.clients.claim());
  } else if (event.data.type === "stop" && owner?.id === source.id) {
    owner = undefined;
  }
});

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)));

async function wireSize(body, type) {
  if (!COMPRESSED.test(type) || typeof CompressionStream === "undefined") return body.byteLength;
  const stream = new Blob([body]).stream().pipeThrough(new CompressionStream("gzip"));
  return (await new Response(stream).arrayBuffer()).byteLength;
}

/** Whether a request belongs to the running comparison: the page's own, or its workers'. */
async function ours(event) {
  if (!owner) return false;
  if (!(await self.clients.get(owner.id))) {
    owner = undefined;
    return false;
  }
  if (event.clientId === owner.id) return true;
  const client = event.clientId && (await self.clients.get(event.clientId));
  return client?.type === "worker";
}

async function handle(event) {
  if (!(await ours(event))) return fetch(event.request);
  const { id, run, folio, left, profile } = owner;
  const url = new URL(event.request.url);
  const side = url.pathname.startsWith(folio)
    ? "right"
    : left.some((path) => url.pathname.startsWith(path))
      ? "left"
      : undefined;
  // The page's own files (a script chunk of the site), on neither side.
  if (!side) return fetch(event.request);
  if (profile.rtt) await sleep(profile.rtt);
  const response = await fetch(event.request, { cache: "no-store" });
  const body = await response.arrayBuffer();
  const type = response.headers.get("content-type") ?? "";
  const counted = wireSize(body, type).then(async (bytes) => {
    const page = await self.clients.get(id);
    page?.postMessage({ type: "bytes", run, side, path: url.pathname, bytes });
    return bytes;
  });
  if (profile.kbps) {
    // One link per side: a file waits for the ones before it on the same side.
    const bytes = await counted;
    const start = Math.max(performance.now(), links[side]);
    links[side] = start + (bytes * 8) / profile.kbps;
    await sleep(links[side] - performance.now());
  } else {
    event.waitUntil(counted);
  }
  // The body is already decoded: the headers that describe the encoding no longer apply.
  const headers = new Headers(response.headers);
  headers.delete("content-encoding");
  headers.delete("content-length");
  return new Response(body, { status: response.status, statusText: response.statusText, headers });
}

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (!owner || url.origin !== self.location.origin) return;
  if (!PATHS.some((path) => url.pathname.startsWith(path))) return;
  event.respondWith(handle(event));
});
