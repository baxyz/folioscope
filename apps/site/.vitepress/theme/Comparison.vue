<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  The same prototype, side by side: another way to show it on a site, and Folioscope. Play starts
  both at once, in the visitor's own browser, on the network they pick. What can be measured from
  here is shown; what cannot is said.
-->
<template>
  <div class="compare">
    <div class="pickers">
      <label>
        <span>Against</span>
        <select v-model="chosen" name="scenario">
          <option v-for="s in scenarios" :key="s.id" :value="s.id">{{ s.title }}</option>
        </select>
      </label>
      <label>
        <span>Network</span>
        <select v-model="network" :disabled="worker !== 'on'" name="network">
          <option v-for="p in profiles" :key="p.id" :value="p.id">{{ p.label }}</option>
        </select>
      </label>
    </div>

    <p class="intro">{{ scenario.about }}</p>
    <p v-if="worker === 'off'" class="warning">
      This browser did not start the page's service worker (a private window may refuse it): the
      network stays yours, and only the page's own requests are counted.
    </p>

    <div class="actions">
      <button class="play" type="button" :disabled="worker === 'starting'" @click="play">
        {{ started ? "Play again" : "Play" }}
      </button>
      <span class="hint"
        >Starts both at once, as a first visit: nothing from your cache. Playing again reloads the
        page.</span
      >
    </div>

    <div class="sides" :class="{ phone: scenario.phone }">
      <section
        class="side"
        data-side="left"
        :data-ms="sides.left.shown === null ? undefined : Math.round(sides.left.shown)"
        :data-bytes="sides.left.bytes"
      >
        <h3>{{ leftTitle }}</h3>
        <div class="stage">
          <template v-if="started">
            <iframe
              v-if="left.kind === 'figma'"
              :key="run"
              :src="left.embed"
              :title="`${scenario.title}, Figma embed`"
              allowfullscreen
              @load="shown('left')"
            ></iframe>
            <folioscope-player
              v-else-if="left.kind === 'raw'"
              :key="`raw-${run}`"
              :src="left.src"
              :controls.attr="''"
              @folioscope:navigate="shown('left')"
              @folioscope:progress="stage = $event.detail.stage"
              @folioscope:error="fail('left', $event.detail.error)"
            >
              <!-- A raw file's text needs its font: fetched and counted too. -->
              <folioscope-font :src="left.font"></folioscope-font>
            </folioscope-player>
            <video
              v-else-if="left.kind === 'video'"
              :key="`video-${run}`"
              :src="left.src"
              muted
              autoplay
              loop
              playsinline
              controls
              @loadeddata="shown('left')"
              @error="fail('left', 'the video could not be played')"
            ></video>
            <div v-else class="images">
              <img
                v-for="image in left.images"
                :key="`${image.src}-${run}`"
                :src="image.src"
                :alt="image.alt"
                @load="imageLoaded"
                @error="fail('left', `${image.src} could not be loaded`)"
              />
            </div>
          </template>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>First screen</dt>
            <dd>
              {{ sides.left.error ? "failed" : time(sides.left.shown) }}
              <small>({{ sides.left.error ?? leftEvent }})</small>
            </dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd v-if="left.kind === 'figma'">
              not countable from here
              <small
                >(another site's iframe): about {{ left.megabytes }} MB in
                {{ left.requests }} requests when we measured</small
              >
            </dd>
            <dd v-else>
              {{ size(sides.left.bytes) }} <small>{{ files(sides.left.files) }}</small>
            </dd>
          </div>
          <div v-if="left.kind === 'raw'">
            <dt>In your browser</dt>
            <dd>
              {{ sides.left.error ? "failed" : (stages[stage] ?? "–") }}
            </dd>
          </div>
          <div>
            <dt>What you get</dt>
            <dd class="gives">{{ leftGives }}</dd>
          </div>
        </dl>
      </section>

      <section
        class="side"
        data-side="right"
        :data-ms="sides.right.shown === null ? undefined : Math.round(sides.right.shown)"
        :data-bytes="sides.right.bytes"
      >
        <h3>Folioscope</h3>
        <div class="stage">
          <!-- `controls` as an attribute: as a property it would replace the player's own `controls`. -->
          <folioscope-player
            v-if="started"
            :key="run"
            :src="scenario.folio"
            :poster="`${scenario.folio}/poster.webp`"
            :controls.attr="''"
            @folioscope:navigate="shown('right')"
            @folioscope:error="fail('right', $event.detail.error)"
          ></folioscope-player>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>First screen</dt>
            <dd>
              {{ sides.right.error ? "failed" : time(sides.right.shown) }}
              <small>({{ sides.right.error ?? "drawn, links live" }})</small>
            </dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>
              {{ size(sides.right.bytes) }}
              <small
                >{{ files(sides.right.files) }}; the player's 11 KB of JavaScript is already on this
                page</small
              >
            </dd>
          </div>
          <div>
            <dt>What you get</dt>
            <dd class="gives">The prototype, clickable and animated</dd>
          </div>
        </dl>
      </section>
    </div>

    <div class="notes">
      <p>
        <strong>What these numbers are.</strong> Both clocks start when you press Play, in your
        browser. “First screen” is the moment named beside it. “Data” is what crossed the network,
        text files counted as gzip sends them; it keeps counting after the first screen, as the
        player fetches the screens one tap away.
      </p>
      <p>
        <strong>The network.</strong> A profile delays this site's files through the page's service
        worker: each request waits a round trip, and each side gets a link of the profile's speed of
        its own, files queued one after another, like the profiles of a browser's developer tools.
        Every Play fetches every file again, and empties the full player's cache of bakes, so each
        run is a first visit.
        <template v-if="left.kind === 'figma'">
          Figma's embed is never slowed: a page cannot reach another site's iframe.</template
        >
      </p>
      <p v-if="left.kind === 'figma'">
        <strong>Figma's data.</strong> We loaded the embed once in Chromium with an empty cache
        (2026-11-05): {{ left.megabytes }} MB in {{ left.requests }} requests, the same on a second
        run. It is a one-off measurement from a headless browser, not a promise about yours.
      </p>
      <p v-else>
        <strong>The material.</strong> All of it comes from one test file,
        <code>smart-animate.fig</code>
        (two screens, a smart animate between them): the raw file, its bake, a recording of the
        baked prototype (Chromium, 375 × 812), and each screen as a PNG at 2x, as a designer would
        export them.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";

type Left =
  | { kind: "figma"; embed: string; megabytes: number; requests: number }
  | { kind: "raw"; src: string; font: string }
  | { kind: "video"; src: string }
  | { kind: "images"; images: { src: string; alt: string }[] };

interface Scenario {
  id: string;
  title: string;
  about: string;
  left: Left;
  folio: string;
  /** A phone-sized prototype: narrower, taller stages. */
  phone?: boolean;
}

const smartAnimate = "/folio/smart-animate.folio";

const scenarios: Scenario[] = [
  {
    id: "co2",
    title: "Figma's embed",
    about:
      "A dashboard prototype of four screens, made in Figma for a freight start-up: the same file, as Figma's own embed and baked by Folioscope.",
    left: {
      kind: "figma",
      embed:
        "https://www.figma.com/embed?embed_host=share&url=https%3A%2F%2Fwww.figma.com%2Fproto%2FAFb4Uyz4Z98S1E6WBnI4gF%2FCO%25E2%2582%2582%3Fnode-id%3D501-9511%26node-type%3DCANVAS%26scaling%3Dcontain%26content-scaling%3Dresponsive%26page-id%3D101%253A4709%26starting-point-node-id%3D501%253A9511",
      megabytes: 23.4,
      requests: 240,
    },
    folio: "/examples/co2.folio",
  },
  {
    id: "raw",
    title: "The raw .fig, baked in the browser",
    about:
      "The same player can take the design file itself: it loads the full player, 6 MB of WebAssembly, and the fonts of the file's text, and bakes the file in your browser. Here, against the bake made once at build time.",
    left: {
      kind: "raw",
      src: "/comparison/smart-animate.fig",
      font: "/comparison/Inter-Latin.woff",
    },
    folio: smartAnimate,
    phone: true,
  },
  {
    id: "video",
    title: "A screen recording",
    about:
      "A common stand-in for a prototype: a video of someone clicking through it. Here, a recording of the same prototype, opening its details and coming back.",
    left: { kind: "video", src: "/comparison/smart-animate.webm" },
    folio: smartAnimate,
    phone: true,
  },
  {
    id: "images",
    title: "One PNG per screen",
    about:
      "The plainest stand-in: each screen exported as a picture, at 2x for sharp screens. Here, the two screens of the same prototype.",
    left: {
      kind: "images",
      images: [
        { src: "/comparison/smart-animate-home.png", alt: "The home screen" },
        { src: "/comparison/smart-animate-details.png", alt: "The details screen" },
      ],
    },
    folio: smartAnimate,
    phone: true,
  },
];

const profiles = [
  { id: "yours", label: "Your connection", rtt: 0, kbps: 0 },
  { id: "fast-4g", label: "Fast 4G (9 Mb/s, 170 ms)", rtt: 170, kbps: 9000 },
  { id: "slow-4g", label: "Slow 4G (1.6 Mb/s, 560 ms)", rtt: 560, kbps: 1600 },
  { id: "3g", label: "3G (400 kb/s, 2 s)", rtt: 2000, kbps: 400 },
];

const stages: Record<string, string> = {
  download: "downloading the file",
  fonts: "loading fonts",
  cache: "read from its cache",
  bake: "baked",
};

const chosen = ref("co2");
const network = ref("yours");
const scenario = computed(() => scenarios.find((s) => s.id === chosen.value) ?? scenarios[0]!);
const profile = computed(() => profiles.find((p) => p.id === network.value) ?? profiles[0]!);
const left = computed(() => scenario.value.left);
const started = ref(false);
const running = ref(false);
const run = ref(0);
const now = ref(0);
const stage = ref("");
/** The page's service worker: what slows and counts the network. */
const worker = ref<"starting" | "on" | "off">("starting");

interface Side {
  shown: number | null;
  /** Why it could not show: the run stops waiting for it. */
  error: string | null;
  bytes: number;
  files: number;
}
const sides = reactive<{ left: Side; right: Side }>({
  left: { shown: null, error: null, bytes: 0, files: 0 },
  right: { shown: null, error: null, bytes: 0, files: 0 },
});

const leftTitle = computed(
  () =>
    ({
      figma: "Figma's embed",
      raw: "The raw file, baked here",
      video: "A screen recording",
      images: "One PNG per screen",
    })[left.value.kind],
);
const leftEvent = computed(
  () =>
    ({
      figma: "the iframe's load event",
      raw: "drawn, links live",
      video: "first frame decoded",
      images: "every image loaded",
    })[left.value.kind],
);
const leftGives = computed(
  () =>
    ({
      figma: "The prototype, run by Figma's engine, from Figma's servers",
      raw: "The prototype, clickable and animated, once baked",
      video: "A film of one path through it: nothing to click",
      images: "Pictures of the screens: nothing to click, no transitions",
    })[left.value.kind],
);

let t0 = 0;
let frame = 0;
/**
 * Whether this document already ran a comparison: its images are then in the document's own
 * cache, which no service worker sees, so the next run starts from a fresh page.
 */
let played = false;
let imagesLeft = 0;
let registration: ServiceWorkerRegistration | undefined;

const seconds = (ms: number) =>
  ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;
const time = (at: number | null) =>
  at === null ? (started.value ? seconds(now.value) : "–") : seconds(at);
const size = (bytes: number) =>
  !started.value
    ? "–"
    : bytes < 1024 * 1024
      ? `${Math.round(bytes / 1024)} KB`
      : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
const files = (count: number) => (count ? `(${count} ${count === 1 ? "file" : "files"})` : "");

function tick() {
  now.value = performance.now() - t0;
  if (running.value) frame = requestAnimationFrame(tick);
}

function shown(side: "left" | "right") {
  if (sides[side].shown !== null || sides[side].error !== null) return;
  sides[side].shown = performance.now() - t0;
  if (worker.value !== "on") countFromTimings();
  finishIfDone();
}

function fail(side: "left" | "right", error: unknown) {
  if (sides[side].shown !== null || sides[side].error !== null) return;
  sides[side].error = error instanceof Error ? error.message : String(error);
  finishIfDone();
}

function finishIfDone() {
  const done = (s: Side) => s.shown !== null || s.error !== null;
  if (done(sides.left) && done(sides.right)) running.value = false;
}

function resetSides() {
  for (const side of [sides.left, sides.right]) {
    Object.assign(side, { shown: null, error: null, bytes: 0, files: 0 });
  }
}

// Another choice stops the run on screen: Play starts the new one.
watch([chosen, network], () => {
  cancelAnimationFrame(frame);
  running.value = false;
  started.value = false;
  resetSides();
});

function imageLoaded() {
  if (--imagesLeft === 0) shown("left");
}

/** The paths of the left side's files: the full player's scripts and WebAssembly for a raw file. */
function leftPaths() {
  return left.value.kind === "raw" ? ["/comparison/", "/assets/"] : ["/comparison/"];
}

/** Without the service worker: the page's own requests since Play, from resource timing. */
function countFromTimings() {
  const entries = performance
    .getEntriesByType("resource")
    .filter((e) => e.startTime >= t0) as PerformanceResourceTiming[];
  for (const side of ["left", "right"] as const) {
    const mine = entries.filter((e) => {
      const path = new URL(e.name).pathname;
      const right = path.startsWith(scenario.value.folio);
      return side === "right" ? right : !right && leftPaths().some((p) => path.startsWith(p));
    });
    sides[side].bytes = mine.reduce((sum, e) => sum + (e.transferSize || e.encodedBodySize), 0);
    sides[side].files = mine.length;
  }
}

function onWorkerMessage(event: MessageEvent) {
  const data = event.data as { type: string; run: number; side: "left" | "right"; bytes: number };
  if (data.type !== "bytes" || data.run !== run.value) return;
  sides[data.side].bytes += data.bytes;
  sides[data.side].files++;
}

/** Tells the service worker a run starts, and waits until it has heard. */
async function arm(): Promise<boolean> {
  const controller = navigator.serviceWorker?.controller;
  if (!controller) return false;
  const { rtt, kbps } = profile.value;
  return new Promise((resolve) => {
    const timeout = setTimeout(() => resolve(false), 2000);
    const ready = (event: MessageEvent) => {
      if (event.data?.type !== "ready" || event.data.run !== run.value) return;
      clearTimeout(timeout);
      navigator.serviceWorker.removeEventListener("message", ready);
      resolve(true);
    };
    navigator.serviceWorker.addEventListener("message", ready);
    controller.postMessage({
      type: "run",
      run: run.value,
      folio: scenario.value.folio,
      left: leftPaths(),
      profile: { rtt, kbps },
    });
  });
}

async function play() {
  if (played) {
    const query = new URLSearchParams({
      against: scenario.value.id,
      network: profile.value.id,
      play: "",
    });
    location.replace(`${location.pathname}?${query}`);
    return;
  }
  played = true;
  cancelAnimationFrame(frame);
  started.value = false;
  await nextTick();
  run.value++;
  resetSides();
  stage.value = "";
  imagesLeft = left.value.kind === "images" ? left.value.images.length : 0;
  // The full player keeps its bakes in the Cache API: forgotten, so it bakes again.
  if (left.value.kind === "raw") await globalThis.caches?.delete("folioscope").catch(() => false);
  if (worker.value === "on" && !(await arm())) worker.value = "off";
  running.value = true;
  t0 = performance.now();
  now.value = 0;
  started.value = true;
  frame = requestAnimationFrame(tick);
}

onMounted(async () => {
  // `?against=raw&network=slow-4g&play`: the choices, and whether to start at once.
  const query = new URLSearchParams(location.search);
  const against = query.get("against");
  if (scenarios.some((s) => s.id === against)) chosen.value = against!;
  const speed = query.get("network");
  if (profiles.some((p) => p.id === speed)) network.value = speed!;
  await startWorker();
  if (query.has("play")) void play();
});

async function startWorker() {
  const container = navigator.serviceWorker;
  if (!container) {
    worker.value = "off";
    return;
  }
  container.addEventListener("message", onWorkerMessage);
  // Messages from the worker wait in a queue until this, in Firefox when listening this way.
  container.startMessages();
  try {
    // Scope `/`: the full player's worker, under /assets/, has to be reached too.
    registration = await container.register("/comparison-sw.js", { scope: "/" });
    if (!container.controller) {
      // A page loaded before the worker, or past it (a hard reload; Firefox at times), is not
      // controlled: the active worker is asked to claim it.
      const changed = new Promise((resolve, reject) => {
        container.addEventListener("controllerchange", resolve, { once: true });
        setTimeout(reject, 5000);
      });
      (await container.ready).active?.postMessage({ type: "claim" });
      await changed;
    }
    worker.value = "on";
  } catch {
    worker.value = "off";
  }
}

onBeforeUnmount(() => {
  cancelAnimationFrame(frame);
  navigator.serviceWorker?.removeEventListener("message", onWorkerMessage);
  navigator.serviceWorker?.controller?.postMessage({ type: "stop" });
  // Only this page needs it: other pages are not slowed down by a fetch handler.
  void registration?.unregister();
});
</script>

<style scoped>
.compare {
  margin: 24px 0;
}
.pickers {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 24px;
  margin-bottom: 12px;
}
.pickers label {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pickers select {
  max-width: 100%;
  padding: 4px 8px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1);
}
.intro {
  margin: 0 0 12px;
  color: var(--vp-c-text-2);
}
.warning {
  margin: 0 0 12px;
  color: var(--vp-c-warning-1);
  font-size: 14px;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.play {
  padding: 8px 24px;
  border-radius: 20px;
  background: var(--vp-button-brand-bg);
  color: var(--vp-button-brand-text);
  font-weight: 600;
}
.play:hover {
  background: var(--vp-button-brand-hover-bg);
}
.play:disabled {
  opacity: 0.6;
}
.hint {
  color: var(--vp-c-text-3);
  font-size: 13px;
}
.sides {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}
.side h3 {
  margin: 0 0 8px;
  font-size: 16px;
}
.stage {
  position: relative;
  height: 340px;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
}
.phone .stage {
  display: flex;
  justify-content: center;
  height: 520px;
}
.stage iframe {
  width: 100%;
  height: 100%;
  border: 0;
}
.stage folioscope-player {
  display: block;
  width: 100%;
}
/* A 375 × 812 screen in the stage's height. */
.phone .stage folioscope-player,
.phone .stage video {
  width: 240px;
  max-width: 100%;
  align-self: flex-start;
}
.stage video {
  display: block;
  height: 100%;
}
.images {
  display: flex;
  gap: 8px;
  height: 100%;
  padding: 8px;
}
.images img {
  min-width: 0;
  max-height: 100%;
  object-fit: contain;
}
.idle {
  margin: 0;
  padding: 16px;
  color: var(--vp-c-text-3);
}
.stats {
  margin: 8px 0 0;
}
.stats div {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  padding: 4px 0;
  border-bottom: 1px solid var(--vp-c-divider);
  font-size: 14px;
}
.stats dt {
  flex: 0 0 9em;
  color: var(--vp-c-text-2);
}
.stats dd {
  flex: 1 1 12em;
  margin: 0;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
.stats dd.gives {
  font-weight: 400;
}
.stats small {
  font-weight: 400;
  color: var(--vp-c-text-3);
}
.notes {
  margin-top: 16px;
  color: var(--vp-c-text-2);
  font-size: 14px;
}
@media (max-width: 720px) {
  .sides {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
