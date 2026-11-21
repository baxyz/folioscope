<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Features by status, with what each one still needs: generated from data/features.ts. -->
<template>
  <section v-for="section in sections" :key="section.status">
    <h2 :id="section.status">{{ section.title }} ({{ section.items.length }})</h2>
    <p>{{ section.about }}</p>
    <ul>
      <li v-for="item in section.items" :key="item.key">
        <strong>{{ item.name }}</strong>
        <span v-if="item.formats"> — {{ item.formats }}</span>
        <span v-if="item.missing">: {{ item.missing }}</span>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import { FEATURES, FORMATS, type Status } from "../../data/features.ts";

interface Item {
  key: string;
  name: string;
  formats?: string;
  missing?: string;
}

/** One entry per feature and status, the formats sharing it (and what they miss) grouped. */
function items(status: Status["status"]): Item[] {
  const out: Item[] = [];
  for (const feature of FEATURES) {
    if ("all" in feature.formats) {
      const s = feature.formats.all;
      if (s.status === status)
        out.push({
          key: feature.id,
          name: feature.name,
          missing: "missing" in s ? s.missing : undefined,
        });
      continue;
    }
    const groups = new Map<string, string[]>();
    for (const f of FORMATS) {
      const s = feature.formats[f.id];
      if (s?.status !== status) continue;
      const missing = "missing" in s ? s.missing : "";
      groups.set(missing, [...(groups.get(missing) ?? []), f.name]);
    }
    for (const [missing, formats] of groups) {
      out.push({
        key: `${feature.id}:${missing}`,
        name: feature.name,
        formats: formats.join(", "),
        missing: missing || undefined,
      });
    }
  }
  return out;
}

const sections = [
  {
    status: "works",
    title: "Works",
    about: "Tested, and checked in the design tool or on a real file.",
    items: items("works"),
  },
  {
    status: "unverified",
    title: "Works, not validated yet",
    about:
      "Tested on files we made, not yet checked in the design tool or on a real file. A file that uses the feature, shared under a free licence, would validate it.",
    items: items("unverified"),
  },
  {
    status: "todo",
    title: "To do",
    about: "Not done yet, and what it needs.",
    items: items("todo"),
  },
];
</script>
