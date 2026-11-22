<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  What plays, per feature and design format, generated from data/features.ts: one compact table,
  a symbol per cell, its details in a tip on hover or keyboard focus.
-->
<template>
  <div class="matrix">
    <ul class="legend">
      <li v-for="(s, key) in SYMBOLS" :key="key">
        <span class="symbol" aria-hidden="true">{{ s.symbol }}</span> {{ s.label }}
        <span class="count">{{ counts[key] }}</span>
      </li>
    </ul>
    <table>
      <thead>
        <tr>
          <th scope="col">Feature</th>
          <th v-for="f in FORMATS" :key="f.id" scope="col" class="format">{{ short[f.id] }}</th>
        </tr>
      </thead>
      <tbody v-for="group in groups" :key="group">
        <tr class="group">
          <th :id="slug(group)" :colspan="FORMATS.length + 1" scope="colgroup">{{ group }}</th>
        </tr>
        <tr v-for="feature in byGroup(group)" :key="feature.id">
          <th scope="row">{{ feature.name }}</th>
          <td v-if="'all' in feature.formats" :colspan="FORMATS.length" class="all">
            <Cell :status="feature.formats.all" label="Every format" />
          </td>
          <td v-for="f in 'all' in feature.formats ? [] : FORMATS" v-else :key="f.id">
            <Cell :status="feature.formats[f.id]" :label="short[f.id]" />
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { defineComponent, h, type PropType } from "vue";
import { FEATURES, FORMATS, type Format, type Status } from "../../data/features.ts";

const SYMBOLS = {
  works: { symbol: "✅", label: "Plays" },
  unverified: { symbol: "🧪", label: "Plays, to validate on a real file" },
  todo: { symbol: "🚧", label: "Not yet" },
  na: { symbol: "—", label: "Not in the format" },
} as const;

const short: Record<Format, string> = {
  fig: "Figma",
  sketch: "Sketch",
  penpot: "Penpot",
  xd: "XD",
};

const groups = [...new Set(FEATURES.map((f) => f.group))];
const byGroup = (group: string) => FEATURES.filter((f) => f.group === group);
const slug = (s: string) => s.toLowerCase().replace(/\W+/g, "-");

const counts = { works: 0, unverified: 0, todo: 0, na: 0 };
for (const feature of FEATURES) {
  const statuses =
    "all" in feature.formats
      ? [feature.formats.all]
      : FORMATS.map((f) => (feature.formats as Partial<Record<Format, Status>>)[f.id]);
  for (const s of statuses) counts[s?.status ?? "na"]++;
}

/** What a cell says in full: its status, its note, and what it still needs. */
function details(s: Status): string {
  const parts: string[] = [SYMBOLS[s.status].label];
  if ("note" in s && s.note) parts.push(s.note);
  if ("missing" in s) parts.push(`Needs ${s.missing}.`);
  return parts.join(". ").replace(/\.\./g, ".");
}

/** One cell: its symbol, and its details in a tip (and for screen readers). */
const Cell = defineComponent({
  props: {
    status: { type: Object as PropType<Status | undefined>, default: undefined },
    label: { type: String, required: true },
  },
  setup(props) {
    return () => {
      const s = props.status ?? ({ status: "na" } as Status);
      const text = details(s);
      // A plain "plays" or "not in the format" says it all; the others have more to say.
      const hasMore = s.status === "unverified" || s.status === "todo" || ("note" in s && !!s.note);
      return h(
        "span",
        {
          class: ["cell", s.status, { more: hasMore }],
          tabindex: hasMore ? 0 : undefined,
          "aria-label": `${props.label}: ${text}`,
        },
        [
          h("span", { "aria-hidden": "true" }, SYMBOLS[s.status].symbol),
          hasMore ? h("span", { class: "tip", role: "tooltip" }, text) : null,
        ],
      );
    };
  },
});
</script>

<style scoped>
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 20px;
  margin: 16px 0;
  padding: 0;
  list-style: none;
  font-size: 14px;
}
.legend li {
  margin: 0;
}
.legend .count {
  margin-left: 2px;
  padding: 0 6px;
  border-radius: 8px;
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-2);
  font-size: 12px;
}
.matrix table {
  display: table;
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}
.matrix th,
.matrix td {
  padding: 5px 10px;
  border: 0;
  border-bottom: 1px solid var(--vp-c-divider);
}
.matrix tr {
  background: transparent !important;
}
.matrix thead th {
  position: sticky;
  top: var(--vp-nav-height);
  z-index: 2;
  background: var(--vp-c-bg);
}
.matrix th.format,
.matrix td {
  width: 76px;
  text-align: center;
}
.matrix tbody th[scope="row"] {
  font-weight: 400;
  text-align: left;
  background: transparent;
}
.matrix tbody tr:hover {
  background: var(--vp-c-bg-soft) !important;
}
.matrix tr.group th {
  padding-top: 20px;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  text-align: left;
  color: var(--vp-c-text-2);
}
.matrix td.all {
  text-align: center;
}
:deep(.cell) {
  position: relative;
  display: inline-block;
  min-width: 24px;
  border-radius: 6px;
  outline-offset: 2px;
}
:deep(.cell.na) {
  color: var(--vp-c-text-3);
}
:deep(.cell.more) {
  cursor: help;
}
:deep(.cell.more:hover),
:deep(.cell.more:focus-visible) {
  background: var(--vp-c-default-soft);
}
:deep(.cell .tip) {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 50%;
  z-index: 10;
  width: max-content;
  max-width: 280px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--vp-c-bg-elv);
  box-shadow: var(--vp-shadow-3);
  color: var(--vp-c-text-1);
  font-size: 13px;
  line-height: 1.45;
  text-align: left;
  transform: translateX(-50%);
  visibility: hidden;
  opacity: 0;
  transition: opacity 0.15s;
  pointer-events: none;
}
:deep(.cell.more:hover .tip),
:deep(.cell.more:focus-visible .tip) {
  visibility: visible;
  opacity: 1;
}
@media (max-width: 640px) {
  .matrix th.format,
  .matrix td {
    width: 44px;
    padding: 5px 4px;
  }
  .matrix th.format {
    font-size: 11px;
  }
}
</style>
