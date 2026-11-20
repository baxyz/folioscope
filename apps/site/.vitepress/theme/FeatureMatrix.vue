<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- What plays, per feature and design format: generated from data/features.ts. -->
<template>
  <div v-for="group in groups" :key="group" class="group">
    <h2 :id="slug(group)">{{ group }}</h2>
    <table>
      <thead>
        <tr>
          <th>Feature</th>
          <th v-for="f in FORMATS" :key="f.id">{{ f.name }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="feature in byGroup(group)" :key="feature.id">
          <td>{{ feature.name }}</td>
          <template v-if="'all' in feature.formats">
            <td :colspan="FORMATS.length" :class="feature.formats.all.status">
              {{ label(feature.formats.all) }}
            </td>
          </template>
          <template v-else>
            <td v-for="f in FORMATS" :key="f.id" :class="cell(feature, f.id)?.status ?? 'na'">
              {{ label(cell(feature, f.id)) }}
            </td>
          </template>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { FEATURES, FORMATS, type Feature, type Format, type Status } from "../../data/features.ts";

const groups = [...new Set(FEATURES.map((f) => f.group))];
const byGroup = (group: string) => FEATURES.filter((f) => f.group === group);
const slug = (s: string) => s.toLowerCase().replace(/\W+/g, "-");
const cell = (feature: Feature, format: Format): Status | undefined =>
  "all" in feature.formats ? feature.formats.all : feature.formats[format];
const label = (s: Status | undefined): string => {
  if (!s || s.status === "na") return "Not in the format";
  const note = "note" in s && s.note ? ` (${s.note})` : "";
  if (s.status === "works") return `Plays${note}`;
  if (s.status === "unverified") return `Plays, unverified${note}`;
  return "Not yet";
};
</script>

<style scoped>
td.works {
  color: var(--vp-c-green-1);
}
td.unverified {
  color: var(--vp-c-yellow-1);
}
td.todo,
td.na {
  color: var(--vp-c-text-3);
}
</style>
