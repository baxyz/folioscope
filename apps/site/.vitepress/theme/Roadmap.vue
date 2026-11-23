<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  Where Folioscope stands, generated from data/features.ts: a summary per format, what is left to
  do, what waits for a real file to validate it, and what is done (folded).
-->
<template>
  <div class="roadmap">
    <div class="tiles">
      <div class="tile works">
        <strong>{{ total.works }}</strong>
        <span>play and are validated</span>
      </div>
      <div class="tile unverified">
        <strong>{{ total.unverified }}</strong>
        <span>play, to validate on a real file</span>
      </div>
      <div class="tile todo">
        <strong>{{ total.todo }}</strong>
        <span>left to do</span>
      </div>
    </div>

    <div class="bars">
      <div v-for="f in perFormat" :key="f.id" class="bar-row">
        <span class="bar-name">{{ f.name }}</span>
        <span
          class="bar"
          role="img"
          :aria-label="`${f.name}: ${f.works} validated, ${f.unverified} to validate, ${f.todo} to do`"
        >
          <span class="works" :style="{ flex: f.works }"></span>
          <span class="unverified" :style="{ flex: f.unverified }"></span>
          <span class="todo" :style="{ flex: f.todo }"></span>
        </span>
        <span class="bar-count"
          >{{ f.works + f.unverified }} / {{ f.works + f.unverified + f.todo }}</span
        >
      </div>
    </div>

    <h2 id="to-do">To do</h2>
    <p class="about">What the format has and the player does not play yet, and what it needs.</p>
    <div class="cards">
      <article v-for="item in todo" :key="item.key" class="card">
        <header>
          <span class="group">{{ item.group }}</span>
          <span v-for="f in item.formats" :key="f" class="badge" :data-format="f">{{
            short[f]
          }}</span>
        </header>
        <h3>{{ item.name }}</h3>
        <p>Needs {{ item.missing }}.</p>
      </article>
    </div>

    <h2 id="to-validate">To validate</h2>
    <p class="about">
      These play, tested on files we made. A real file that uses them, shared under a free licence,
      would prove them against the design tool's own rendering.
    </p>
    <div class="cards formats">
      <article v-for="f in toValidate" :key="f.id" class="card">
        <header>
          <span class="badge" :data-format="f.id">{{ short[f.id] }}</span>
          <span class="count">{{ f.items.length }}</span>
        </header>
        <p v-if="f.usual" class="usual">Needs {{ f.usual }}:</p>
        <ul class="chips">
          <li
            v-for="item in f.items"
            :key="item.name"
            :title="item.missing === f.usual ? undefined : `Needs ${item.missing}`"
            :class="{ own: item.missing !== f.usual }"
          >
            {{ item.name }}
          </li>
        </ul>
        <ul v-if="f.items.some((i) => i.missing !== f.usual)" class="own-needs">
          <li v-for="item in f.items.filter((i) => i.missing !== f.usual)" :key="item.name">
            <strong>{{ item.name }}</strong>: needs {{ item.missing }}.
          </li>
        </ul>
      </article>
    </div>

    <h2 id="done">Done</h2>
    <details class="done">
      <summary>{{ total.works }} features play and are validated</summary>
      <div v-for="group in doneGroups" :key="group.name" class="done-group">
        <h3>{{ group.name }}</h3>
        <ul class="chips">
          <li v-for="item in group.items" :key="item">{{ item }}</li>
        </ul>
      </div>
    </details>
  </div>
</template>

<script setup lang="ts">
import { FEATURES, FORMATS, type Format, type Status } from "../../data/features.ts";

const short: Record<Format, string> = {
  fig: "Figma",
  sketch: "Sketch",
  penpot: "Penpot",
  xd: "XD",
};

/** Each feature's status per format; a feature that does not depend on the format counts once. */
const cells = FEATURES.flatMap((feature) =>
  "all" in feature.formats
    ? [{ feature, format: undefined as Format | undefined, status: feature.formats.all }]
    : FORMATS.flatMap((f) => {
        const status = (feature.formats as Partial<Record<Format, Status>>)[f.id];
        return status ? [{ feature, format: f.id as Format | undefined, status }] : [];
      }),
);

const total = { works: 0, unverified: 0, todo: 0 };
for (const c of cells) if (c.status.status !== "na") total[c.status.status]++;

const perFormat = FORMATS.map((f) => {
  const mine = cells.filter((c) => c.format === f.id || c.format === undefined);
  const n = (s: Status["status"]) => mine.filter((c) => c.status.status === s).length;
  return {
    id: f.id,
    name: short[f.id],
    works: n("works"),
    unverified: n("unverified"),
    todo: n("todo"),
  };
});

/** What is left to do, one card per feature and need, its formats together. */
const todo: { key: string; name: string; group: string; formats: Format[]; missing: string }[] = [];
for (const c of cells) {
  if (c.status.status !== "todo") continue;
  const missing = c.status.missing;
  const same = todo.find((t) => t.name === c.feature.name && t.missing === missing);
  if (same && c.format) same.formats.push(c.format);
  else
    todo.push({
      key: `${c.feature.id}:${missing}`,
      name: c.feature.name,
      group: c.feature.group,
      formats: c.format ? [c.format] : [],
      missing,
    });
}

/** What waits for a real file, per format, with the need most of them share said once. */
const toValidate = FORMATS.map((f) => {
  const items = cells
    .filter((c) => c.format === f.id && c.status.status === "unverified")
    .map((c) => ({ name: c.feature.name, missing: (c.status as { missing: string }).missing }));
  const tally = new Map<string, number>();
  for (const i of items) tally.set(i.missing, (tally.get(i.missing) ?? 0) + 1);
  const usual = [...tally].sort((a, b) => b[1] - a[1])[0]?.[0];
  return { id: f.id, items, usual };
}).filter((f) => f.items.length);

const doneGroups = [...new Set(FEATURES.map((f) => f.group))]
  .map((name) => ({
    name,
    items: [
      ...new Set(
        cells
          .filter((c) => c.feature.group === name && c.status.status === "works")
          .map((c) => c.feature.name),
      ),
    ],
  }))
  .filter((g) => g.items.length);
</script>

<style scoped>
.tiles {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin: 24px 0 16px;
}
.tile {
  display: flex;
  flex-direction: column;
  padding: 16px;
  border-radius: 12px;
  background: var(--vp-c-bg-soft);
  border-top: 4px solid;
}
.tile strong {
  font-size: 32px;
  line-height: 1.1;
}
.tile span {
  color: var(--vp-c-text-2);
  font-size: 14px;
}
.tile.works,
.bar .works {
  border-color: var(--vp-c-green-2);
  background-color: var(--vp-c-green-soft);
}
.tile.unverified,
.bar .unverified {
  border-color: var(--vp-c-yellow-2);
  background-color: var(--vp-c-yellow-soft);
}
.tile.todo,
.bar .todo {
  border-color: var(--vp-c-gray-2);
  background-color: var(--vp-c-default-soft);
}
.bar .works {
  background-color: var(--vp-c-green-2);
}
.bar .unverified {
  background-color: var(--vp-c-yellow-2);
}
.bar .todo {
  background-color: var(--vp-c-gray-2);
}
.bars {
  display: grid;
  gap: 8px;
  margin-bottom: 8px;
}
.bar-row {
  display: grid;
  grid-template-columns: 64px minmax(0, 1fr) 64px;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}
.bar {
  display: flex;
  height: 10px;
  overflow: hidden;
  border-radius: 5px;
  gap: 2px;
}
.bar-count {
  color: var(--vp-c-text-2);
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.about {
  color: var(--vp-c-text-2);
}
.cards {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.card {
  padding: 14px 16px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
}
.card header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.card h3 {
  margin: 8px 0 4px;
  font-size: 16px;
}
.card p {
  margin: 0;
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 1.5;
}
.group {
  margin-right: auto;
  color: var(--vp-c-text-3);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.count {
  color: var(--vp-c-text-3);
  font-size: 13px;
}
.badge {
  padding: 1px 8px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 600;
  background: var(--vp-c-default-soft);
}
.badge[data-format="fig"] {
  background: #e0f2fe;
  color: #075985;
}
.badge[data-format="sketch"] {
  background: #fef3c7;
  color: #92400e;
}
.badge[data-format="penpot"] {
  background: #ede9fe;
  color: #5b21b6;
}
.badge[data-format="xd"] {
  background: #fce7f3;
  color: #9d174d;
}
.usual {
  margin: 10px 0 6px !important;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
}
.chips li {
  margin: 0;
  padding: 2px 10px;
  border-radius: 12px;
  background: var(--vp-c-default-soft);
  font-size: 13px;
}
.chips li.own {
  outline: 1px dashed var(--vp-c-yellow-2);
}
.own-needs {
  margin: 10px 0 0;
  padding-left: 18px;
  color: var(--vp-c-text-2);
  font-size: 13px;
}
.own-needs li {
  margin: 4px 0;
}
.done {
  padding: 12px 16px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
}
.done summary {
  cursor: pointer;
  font-weight: 600;
}
.done-group h3 {
  margin: 16px 0 0;
  font-size: 14px;
}
@media (max-width: 640px) {
  .tiles,
  .cards {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
