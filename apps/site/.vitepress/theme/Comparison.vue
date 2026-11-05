<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  The same prototype, side by side: Figma's embed and Folioscope. Play starts both at once, in the
  visitor's own browser. What can be measured from here is shown; what cannot is said.
-->
<template>
  <div class="compare">
    <div v-if="scenarios.length > 1" class="picker">
      <label for="scenario">Scenario</label>
      <select id="scenario" v-model="chosen" :disabled="running">
        <option v-for="(s, i) in scenarios" :key="s.id" :value="i">{{ s.title }}</option>
      </select>
    </div>

    <p class="intro">{{ scenario.about }}</p>

    <div class="actions">
      <button class="play" type="button" @click="play">
        {{ started ? "Play again" : "Play" }}
      </button>
      <span class="hint">Starts both at once, with an empty page behind each.</span>
    </div>

    <div class="sides">
      <section class="side">
        <h3>Figma's embed</h3>
        <div class="stage">
          <iframe
            v-if="started"
            :key="run"
            :src="scenario.embed"
            :title="`${scenario.title}, Figma embed`"
            allowfullscreen
            @load="onFigmaLoad"
          ></iframe>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>Iframe loaded</dt>
            <dd>{{ figma.loaded === null ? (started ? clock : "–") : seconds(figma.loaded) }}</dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>
              not countable from here
              <small
                >(a cross-origin iframe): about {{ scenario.figmaMegabytes }} MB when we
                measured</small
              >
            </dd>
          </div>
        </dl>
      </section>

      <section class="side">
        <h3>Folioscope</h3>
        <div class="stage">
          <!-- `controls` as an attribute: as a property it would replace the player's own `controls`. -->
          <folioscope-player
            v-if="started"
            :key="run"
            ref="player"
            :src="scenario.folio"
            :poster="`${scenario.folio}/poster.webp`"
            :controls.attr="''"
          ></folioscope-player>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>First screen, clickable</dt>
            <dd>{{ ours.shown === null ? (started ? clock : "–") : seconds(ours.shown) }}</dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>
              {{
                ours.kilobytes === null
                  ? "–"
                  : ours.cached
                    ? "from your cache"
                    : `${ours.kilobytes} KB`
              }}
              <small>(the screens; the player's 11 KB of JavaScript is already on this page)</small>
            </dd>
          </div>
        </dl>
      </section>
    </div>

    <div class="notes">
      <p>
        <strong>What these numbers are.</strong> “Iframe loaded” is the browser's
        <code>load</code> event for Figma's page: its prototype may still be loading behind it,
        which this page cannot see. “First screen, clickable” is when the player has drawn the first
        screen with its links live. Both clocks start when you press Play, in your browser, on your
        connection.
      </p>
      <p>
        <strong>Figma's data.</strong> We loaded the embed once in Chromium with an empty cache
        (2026-11-05): {{ scenario.figmaMegabytes }} MB in {{ scenario.figmaRequests }} requests, the
        same on a second run. It is a one-off measurement from a headless browser, not a promise
        about yours.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref } from "vue";

interface Scenario {
  id: string;
  title: string;
  about: string;
  embed: string;
  folio: string;
  figmaMegabytes: number;
  figmaRequests: number;
}

const scenarios: Scenario[] = [
  {
    id: "co2",
    title: "CO₂ Emissions Optimizer",
    about:
      "A dashboard prototype of four screens, made in Figma for a freight start-up: the same file, as Figma's own embed and baked by Folioscope.",
    embed:
      "https://www.figma.com/embed?embed_host=share&url=https%3A%2F%2Fwww.figma.com%2Fproto%2FAFb4Uyz4Z98S1E6WBnI4gF%2FCO%25E2%2582%2582%3Fnode-id%3D501-9511%26node-type%3DCANVAS%26scaling%3Dcontain%26content-scaling%3Dresponsive%26page-id%3D101%253A4709%26starting-point-node-id%3D501%253A9511",
    folio: "/examples/co2.folio",
    figmaMegabytes: 23.4,
    figmaRequests: 240,
  },
];

const chosen = ref(0);
const scenario = computed(() => scenarios[chosen.value]!);
const started = ref(false);
const running = ref(false);
const run = ref(0);
const clock = ref("0.0 s");
const player = ref<HTMLElement | null>(null);

const figma = reactive<{ loaded: number | null }>({ loaded: null });
const ours = reactive<{ shown: number | null; kilobytes: number | null; cached: boolean }>({
  shown: null,
  kilobytes: null,
  cached: false,
});

let t0 = 0;
let frame = 0;

const seconds = (ms: number) =>
  ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;

function tick() {
  clock.value = seconds(performance.now() - t0);
  if (running.value) frame = requestAnimationFrame(tick);
}

function finishIfDone() {
  if (figma.loaded !== null && ours.shown !== null) running.value = false;
}

function onFigmaLoad() {
  figma.loaded = performance.now() - t0;
  finishIfDone();
}

/** What the player fetched for this prototype since Play: same origin, so it can be counted. */
function countOurData() {
  const entries = performance
    .getEntriesByType("resource")
    .filter(
      (e) => e.startTime >= t0 && e.name.includes(scenario.value.folio),
    ) as PerformanceResourceTiming[];
  const transferred = entries.reduce((sum, e) => sum + e.transferSize, 0);
  const body = entries.reduce((sum, e) => sum + e.encodedBodySize, 0);
  ours.cached = transferred === 0 && body > 0;
  ours.kilobytes = Math.round((transferred || body) / 1024);
}

async function play() {
  cancelAnimationFrame(frame);
  started.value = false;
  await nextTick();
  Object.assign(figma, { loaded: null });
  Object.assign(ours, { shown: null, kilobytes: null, cached: false });
  run.value++;
  running.value = true;
  t0 = performance.now();
  started.value = true;
  frame = requestAnimationFrame(tick);
  await nextTick();
  player.value?.addEventListener(
    "folioscope:navigate",
    () => {
      ours.shown = performance.now() - t0;
      countOurData();
      finishIfDone();
    },
    { once: true },
  );
}

onBeforeUnmount(() => cancelAnimationFrame(frame));
</script>

<style scoped>
.compare {
  margin: 24px 0;
}
.picker {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.picker select {
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
.actions {
  display: flex;
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
.stage iframe {
  width: 100%;
  height: 100%;
  border: 0;
}
.stage folioscope-player {
  display: block;
  width: 100%;
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
  flex: 0 0 11em;
  color: var(--vp-c-text-2);
}
.stats dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
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
