<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  Drop a design file to play it. The file never leaves the visitor's browser: the player bakes it
  there (WebAssembly, in a worker) and keeps the result in the browser's cache.
-->
<template>
  <div class="playground">
    <label
      class="drop"
      :class="{ over }"
      @dragover.prevent="over = true"
      @dragleave="over = false"
      @drop.prevent="drop"
    >
      <input type="file" accept=".fig,.sketch,.xd,.penpot" @change="pick" />
      <span v-if="!name"
        >Drop a <code>.fig</code>, <code>.sketch</code>, <code>.xd</code> or
        <code>.penpot</code> file here, or choose one.</span
      >
      <span v-else>{{ name }}: {{ stage }}</span>
    </label>
    <label class="fonts">
      Fonts for its text (optional):
      <input type="file" accept=".ttf,.otf,.ttc,.woff,.woff2" multiple @change="fonts" />
    </label>
    <p v-if="missing.length" class="missing">
      Missing fonts: {{ missing.join(", ") }}. Give them above; the file is baked again.
    </p>
    <p v-if="error" class="error">{{ error }}</p>
    <folioscope-player v-if="src" ref="player" :src="src" controls></folioscope-player>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from "vue";

const src = ref("");
const name = ref("");
const stage = ref("");
const over = ref(false);
const error = ref("");
const missing = ref<string[]>([]);
const player = ref<HTMLElement & { registerFont(font: object): void }>();

function load(file: File | undefined) {
  if (!file) return;
  if (src.value) URL.revokeObjectURL(src.value);
  name.value = file.name;
  stage.value = "reading";
  error.value = "";
  missing.value = [];
  src.value = URL.createObjectURL(file);
}

const pick = (event: Event) => load((event.target as HTMLInputElement).files?.[0]);
const drop = (event: DragEvent) => {
  over.value = false;
  load(event.dataTransfer?.files[0]);
};

async function fonts(event: Event) {
  for (const file of (event.target as HTMLInputElement).files ?? []) {
    player.value?.registerFont({ data: new Uint8Array(await file.arrayBuffer()) });
  }
  missing.value = [];
}

watch(player, async (el) => {
  await nextTick();
  if (!el) return;
  el.addEventListener("folioscope:progress", (e) => {
    stage.value =
      { download: "reading", fonts: "fonts", cache: "from the cache", bake: "baking" }[
        (e as CustomEvent<{ stage: string }>).detail.stage
      ] ?? stage.value;
  });
  el.addEventListener("folioscope:load", () => (stage.value = "playing"));
  el.addEventListener("folioscope:font-missing", (e) => {
    const font = (e as CustomEvent<{ font: string }>).detail.font;
    if (!missing.value.includes(font)) missing.value.push(font);
  });
  el.addEventListener("folioscope:error", (e) => {
    error.value = String((e as CustomEvent<{ error: unknown }>).detail.error);
  });
});
</script>

<style scoped>
.drop {
  display: block;
  padding: 32px 16px;
  border: 2px dashed var(--vp-c-divider);
  border-radius: 12px;
  text-align: center;
  cursor: pointer;
}
.drop.over {
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-bg-soft);
}
.drop input {
  display: none;
}
.fonts {
  display: block;
  margin: 12px 0;
  font-size: 14px;
}
.missing {
  color: var(--vp-c-warning-1);
}
.error {
  color: var(--vp-c-danger-1);
}
folioscope-player {
  display: block;
  max-width: 420px;
  margin: 16px auto;
}
</style>
