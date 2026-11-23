<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  The same prototype, side by side: Figma's own embed and Folioscope. Play starts both at once, in
  the visitor's browser, on their connection. What can be measured from here is shown; what
  cannot is said.
-->
<template>
  <div class="compare">
    <div class="actions">
      <button class="play" type="button" @click="play">
        {{ started ? "Play again" : "Play" }}
      </button>
      <span class="hint"
        >Loads both at once, in your browser, on your connection. Figma's embed comes from
        figma.com, under its own terms.</span
      >
    </div>

    <div class="sides">
      <section
        class="side"
        data-side="figma"
        :data-ms="figma === null ? undefined : Math.round(figma)"
      >
        <h3>Figma's embed</h3>
        <div class="stage">
          <iframe
            v-if="started"
            :key="run"
            :src="EMBED"
            title="CO₂ Emissions Optimizer, Figma embed"
            allowfullscreen
            @load="figma = now()"
          ></iframe>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>Loaded</dt>
            <dd>
              {{ figma === null ? (started ? clock : "–") : seconds(figma) }}
              <small>(the iframe's load event)</small>
            </dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>
              about {{ FIGMA_MB }} MB
              <small
                >in {{ FIGMA_REQUESTS }} requests, measured once: another site's iframe cannot be
                counted from here</small
              >
            </dd>
          </div>
        </dl>
      </section>

      <section
        class="side"
        data-side="folioscope"
        :data-ms="ours === null ? undefined : Math.round(ours)"
        :data-bytes="bytes ?? undefined"
      >
        <h3>Folioscope</h3>
        <div class="stage">
          <folioscope-player
            v-if="started"
            :key="run"
            :src="FOLIO"
            :poster="`${FOLIO}/poster.webp`"
            :controls.attr="''"
            @folioscope:navigate="shown"
          ></folioscope-player>
          <p v-else class="idle">Press Play.</p>
        </div>
        <dl class="stats">
          <div>
            <dt>First screen</dt>
            <dd>
              {{ ours === null ? (started ? clock : "–") : seconds(ours) }}
              <small>(drawn, links live)</small>
            </dd>
          </div>
          <div>
            <dt>Data</dt>
            <dd>
              {{ bytes === null ? "–" : size(bytes) }}
              <small v-if="run > 1" class="again"
                >played again: the screens come from your browser's cache, as on a second
                visit</small
              >
              <small v-else
                >the screens; the player's 11 KB of JavaScript is already on this page</small
              >
            </dd>
          </div>
        </dl>
      </section>
    </div>

    <p class="notes">
      The prototype: a dashboard of four screens, made in Figma for a freight start-up. Both clocks
      start when you press Play. Figma's data was measured once in Chromium with an empty cache
      (2026-11-05): {{ FIGMA_MB }} MB in {{ FIGMA_REQUESTS }} requests, the same on a second run.
    </p>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from "vue";

const EMBED =
  "https://www.figma.com/embed?embed_host=share&url=https%3A%2F%2Fwww.figma.com%2Fproto%2FAFb4Uyz4Z98S1E6WBnI4gF%2FCO%25E2%2582%2582%3Fnode-id%3D501-9511%26node-type%3DCANVAS%26scaling%3Dcontain%26content-scaling%3Dresponsive%26page-id%3D101%253A4709%26starting-point-node-id%3D501%253A9511";
const FOLIO = "/examples/co2.folio";
const FIGMA_MB = 23.4;
const FIGMA_REQUESTS = 240;

const started = ref(false);
const run = ref(0);
const clock = ref("0 ms");
const figma = ref<number | null>(null);
const ours = ref<number | null>(null);
const bytes = ref<number | null>(null);

let t0 = 0;
let frame = 0;
const now = () => performance.now() - t0;
const seconds = (ms: number) =>
  ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;
const size = (n: number) =>
  n < 1024 * 1024 ? `${Math.round(n / 1024)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;

function tick() {
  clock.value = seconds(now());
  if (figma.value === null || ours.value === null) frame = requestAnimationFrame(tick);
}

/** The first screen is up: what the player fetched for it, same origin, so it can be counted. */
function shown() {
  if (ours.value !== null) return;
  ours.value = now();
  const entries = performance
    .getEntriesByType("resource")
    .filter((e) => e.startTime >= t0 && e.name.includes(FOLIO)) as PerformanceResourceTiming[];
  bytes.value = entries.reduce((sum, e) => sum + (e.transferSize || e.encodedBodySize), 0);
}

async function play() {
  cancelAnimationFrame(frame);
  started.value = false;
  await nextTick();
  figma.value = ours.value = bytes.value = null;
  run.value++;
  t0 = performance.now();
  started.value = true;
  frame = requestAnimationFrame(tick);
}

onBeforeUnmount(() => cancelAnimationFrame(frame));
</script>

<style scoped>
.compare {
  margin: 24px 0;
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
  flex: 0 0 7em;
  color: var(--vp-c-text-2);
}
.stats dd {
  flex: 1 1 12em;
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
