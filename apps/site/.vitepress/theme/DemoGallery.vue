<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  The demos, one at a time: a list to pick from, one player, what to try and the markup. The
  address keeps the demo picked (#smart-animate), so a link opens it.
-->
<template>
  <div class="gallery">
    <nav class="list" aria-label="Demos">
      <template v-for="group in groups" :key="group">
        <p class="group">{{ group }}</p>
        <button
          v-for="demo in demos.filter((d) => d.group === group)"
          :key="demo.id"
          type="button"
          class="item"
          :aria-pressed="demo.id === current.id"
          @click="pick(demo.id)"
        >
          <span class="badge" :data-format="demo.format">{{ formats[demo.format] }}</span>
          <span class="name">{{ demo.title }}</span>
        </button>
      </template>
    </nav>

    <section class="show" :class="{ wide: current.wide }" :aria-label="current.title">
      <header class="head">
        <h2>{{ current.title }}</h2>
        <p>{{ current.about }}</p>
      </header>

      <div class="body">
        <div class="stage">
          <folioscope-player
            :key="current.id + (outlined ? '-outlined' : '')"
            ref="player"
            :src="current.src"
            :poster="`${current.src}/poster.webp`"
            :hotspots="outlined ? 'always' : 'on-miss'"
            controls
          ></folioscope-player>
        </div>

        <div class="info">
          <div class="tools">
            <label class="toggle">
              <input v-model="outlined" type="checkbox" />
              Show what is clickable
            </label>
            <span class="now" aria-live="polite">{{ screen ? `On “${screen}”` : "" }}</span>
          </div>

          <h3>Try this</h3>
          <ul>
            <li v-for="step in current.try" :key="step">{{ step }}</li>
          </ul>
          <p class="features">
            <span v-for="tag in current.tags" :key="tag" class="tag">{{ tag }}</span>
          </p>

          <h3>The markup</h3>
          <pre><code>{{ markup }}</code></pre>
          <button type="button" class="copy" @click="copy">{{ copied ? "Copied" : "Copy" }}</button>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

type Format = "fig" | "sketch" | "penpot" | "xd";

interface Demo {
  id: string;
  group: string;
  format: Format;
  title: string;
  about: string;
  src: string;
  try: string[];
  tags: string[];
  /** A desktop prototype: the stage takes the full width. */
  wide?: boolean;
}

const formats: Record<Format, string> = {
  fig: "Figma",
  sketch: "Sketch",
  penpot: "Penpot",
  xd: "XD",
};

const demos: Demo[] = [
  {
    id: "navigation",
    group: "Prototyping",
    format: "fig",
    title: "Navigation",
    about: "Two screens: a button opens the details with a dissolve, another comes back.",
    src: "/folio/navigation.folio",
    try: ["Tap “Open details →”.", "Tap “← Back”, or press Escape."],
    tags: ["click", "dissolve", "back"],
  },
  {
    id: "flows-and-pages",
    group: "Prototyping",
    format: "fig",
    title: "Flows and pages",
    about: "Two pages and three flows, each with its own starting screen.",
    src: "/folio/flows-and-pages.folio",
    try: [
      "Tap “Next →”: the next screen pushes this one away.",
      "In the control bar, pick the “Log in” flow, then the “App” page.",
    ],
    tags: ["flows", "pages", "push", "slide"],
  },
  {
    id: "triggers",
    group: "Prototyping",
    format: "fig",
    title: "Delays, hover, press, keys, swipe",
    about: "Every way a prototype listens: time, the pointer, the keyboard and a drag.",
    src: "/folio/triggers.folio",
    try: [
      "Wait: the splash moves on by itself after 1.5 seconds.",
      "Hover the button, then press and hold it: a screen for each state.",
      "Swipe the grey card, or click the prototype and press ⇧ + →.",
    ],
    tags: ["after delay", "while hovering", "while pressing", "key", "drag"],
  },
  {
    id: "overlay",
    group: "Prototyping",
    format: "fig",
    title: "Overlays",
    about: "A menu over the screen, on a dimmed backdrop, swapped for a longer one.",
    src: "/folio/overlay.folio",
    try: [
      "Tap “Open the menu ↑”: it moves in from the bottom.",
      "Tap “More ↓”: the menu grows, its items moving with smart animate.",
      "Close it with “Close ×” or a click outside.",
    ],
    tags: ["overlay", "swap", "close on click outside"],
  },
  {
    id: "scroll",
    group: "Prototyping",
    format: "fig",
    title: "Scrolling",
    about:
      "A feed that scrolls under a fixed header and tab bar; a gallery with a carousel and a title that sticks.",
    src: "/folio/scroll.folio",
    try: [
      "Scroll the feed: the header and the tabs stay; “Back to top ↑” scrolls back.",
      "Pick the “Gallery flow” in the control bar: swipe the carousel sideways, scroll until “Recent” sticks.",
    ],
    tags: ["scroll", "fixed", "sticky", "horizontal scroll", "scroll to"],
  },
  {
    id: "components",
    group: "Prototyping",
    format: "fig",
    title: "Interactive components",
    about: "Components that change state in place, nested in each other, on a timer.",
    src: "/folio/components.folio",
    try: [
      "Tap the chip: it grows a check, with smart animate.",
      "Flip the switch; open the card and flip the one inside it.",
      "Hover, then press the button; the toast hides after a second.",
    ],
    tags: ["variants", "smart animate", "nested", "after delay"],
  },
  {
    id: "smart-animate",
    group: "Prototyping",
    format: "fig",
    title: "Smart animate",
    about: "Layers with the same name on both screens move from one to the other.",
    src: "/folio/smart-animate.folio",
    try: [
      "Tap “Open the trip →”: the card becomes the header, “Lisbon” grows with it.",
      "Tap “← Back”: the same, backwards.",
    ],
    tags: ["smart animate", "turn", "nested layers"],
  },
  {
    id: "drawing",
    group: "Drawing",
    format: "fig",
    title: "Gradients, shadows, blurs, masks",
    about:
      "Linear, radial, angular and diamond gradients, shadows inside and out, blurs, blend modes, masks, tiled and cropped images.",
    src: "/folio/features.folio",
    try: ["Zoom in: flat layers are SVG, sharp at any size.", "“Open the website →” opens a link."],
    tags: ["gradients", "effects", "blend modes", "masks", "images", "open link"],
  },
  {
    id: "sketch-features",
    group: "Drawing",
    format: "sketch",
    title: "Sketch effects and rich text",
    about:
      "Gradients, shadows, blurs, blend modes, borders and text with styled runs, from a .sketch.",
    src: "/folio/sketch-features.folio",
    try: ["Tap “Open details →”, then “← Back”.", "Pick the second page in the control bar."],
    tags: ["effects", "rich text", "borders", "pages"],
  },
  {
    id: "sketch-navigation",
    group: "Other formats",
    format: "sketch",
    title: "Navigation, from Sketch",
    about: "The navigation demo, made in Sketch's format: the details slide in from the right.",
    src: "/folio/sketch-navigation.folio",
    try: ["Tap “Open details →”, then “← Back”."],
    tags: ["slide", "back"],
  },
  {
    id: "sketch-scroll",
    group: "Other formats",
    format: "sketch",
    title: "Scrolling, from Sketch",
    about: "An artboard taller than its device scrolls under a fixed header.",
    src: "/folio/sketch-scroll.folio",
    try: ["Scroll down to “Open details →”."],
    tags: ["scroll", "fixed"],
  },
  {
    id: "penpot-navigation",
    group: "Other formats",
    format: "penpot",
    title: "Navigation, from Penpot",
    about: "The navigation demo, made in Penpot's format: a named flow and a dissolve.",
    src: "/folio/penpot-navigation.folio",
    try: ["Tap “Open details →”, then “← Back”."],
    tags: ["dissolve", "back"],
  },
  {
    id: "penpot-features",
    group: "Other formats",
    format: "penpot",
    title: "Penpot, every trigger",
    about: "A splash on a timer, hover and press, an overlay toggled by a button, a link.",
    src: "/folio/penpot-features.folio",
    try: [
      "Wait 1.5 seconds on the splash.",
      "Hover “Hover me” for a tip; hold “Hold me”; “•••” opens a menu.",
    ],
    tags: ["after delay", "hover", "press", "overlay", "open link"],
  },
  {
    id: "xd-navigation",
    group: "Other formats",
    format: "xd",
    title: "Navigation, from XD",
    about: "The navigation demo, made in Adobe XD's format.",
    src: "/folio/xd-navigation.folio",
    try: ["Tap “Open details →”, then “← Back”."],
    tags: ["slide", "dissolve"],
  },
  {
    id: "xd-features",
    group: "Other formats",
    format: "xd",
    title: "XD auto-animate and hover",
    about: "A splash on a timer, auto-animate to the details, and a peek on hover.",
    src: "/folio/xd-features.folio",
    try: ["Wait on the splash.", "Hover “Peek: hover me”; tap “Open details →”."],
    tags: ["after delay", "auto-animate", "hover"],
  },
  {
    id: "co2",
    group: "Real files",
    format: "fig",
    title: "A dashboard, from a real Figma file",
    about:
      "Four screens of a freight start-up's emissions dashboard, made in Figma by a designer, baked as it is. The design is its author's, all rights reserved.",
    src: "/examples/co2.folio",
    try: [
      "Explore the dashboard's tabs and buttons.",
      "Compare it with Figma's own embed on the Comparison page.",
    ],
    tags: ["real file", "desktop"],
    wide: true,
  },
];

const groups = [...new Set(demos.map((d) => d.group))];
const currentId = ref(demos[0]!.id);
const current = computed(() => demos.find((d) => d.id === currentId.value) ?? demos[0]!);
const outlined = ref(false);
const copied = ref(false);
const screen = ref("");
const player = ref<HTMLElement | null>(null);

const markup = computed(
  () =>
    `<folioscope-player src="${current.value.src}" poster="${current.value.src}/poster.webp" controls>\n</folioscope-player>`,
);

function pick(id: string) {
  currentId.value = id;
  history.replaceState(null, "", `#${id}`);
}

async function copy() {
  await navigator.clipboard?.writeText(markup.value).catch(() => undefined);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}

const onNavigate = (event: Event) => {
  screen.value = (event as CustomEvent<{ name: string }>).detail.name;
};

// Each new player: what it shows is said under it.
watch(
  [currentId, outlined],
  async () => {
    screen.value = "";
    await nextTick();
    player.value?.addEventListener("folioscope:navigate", onNavigate);
  },
  { flush: "post" },
);

function fromHash() {
  const id = location.hash.slice(1);
  if (demos.some((d) => d.id === id)) currentId.value = id;
}

onMounted(() => {
  fromHash();
  window.addEventListener("hashchange", fromHash);
  player.value?.addEventListener("folioscope:navigate", onNavigate);
});
onBeforeUnmount(() => window.removeEventListener("hashchange", fromHash));
</script>

<style scoped>
.gallery {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  gap: 32px;
  margin: 24px 0 48px;
}
.list {
  position: sticky;
  top: calc(var(--vp-nav-height) + 16px);
  align-self: start;
  max-height: calc(100vh - var(--vp-nav-height) - 32px);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.group {
  margin: 16px 0 4px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--vp-c-text-3);
}
.group:first-child {
  margin-top: 0;
}
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  text-align: left;
  color: var(--vp-c-text-1);
  font-size: 14px;
  line-height: 1.3;
}
.item:hover {
  background: var(--vp-c-bg-soft);
}
.item[aria-pressed="true"] {
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
  font-weight: 600;
}
.badge {
  flex: none;
  min-width: 52px;
  padding: 1px 6px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
  text-align: center;
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-2);
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
.head h2 {
  margin: 0;
  padding: 0;
  border: 0;
  font-size: 24px;
}
.head p {
  margin: 6px 0 16px;
  color: var(--vp-c-text-2);
}
.body {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 28px;
  align-items: start;
}
.wide .body {
  grid-template-columns: minmax(0, 1fr);
}
.stage {
  display: flex;
  justify-content: center;
  padding: 20px;
  border-radius: 16px;
  background: var(--vp-c-bg-soft);
}
.stage folioscope-player {
  width: 270px;
  max-width: 100%;
  border-radius: 20px;
  overflow: hidden;
  box-shadow: 0 8px 30px rgb(0 0 0 / 0.15);
}
.wide .stage folioscope-player {
  width: 100%;
  border-radius: 8px;
}
.tools {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 8px 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--vp-c-divider);
  font-size: 14px;
}
.toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
}
.now {
  color: var(--vp-c-text-2);
}
.info h3 {
  margin: 16px 0 8px;
  font-size: 16px;
}
.info ul {
  margin: 0;
  padding-left: 18px;
}
.info li {
  margin: 4px 0;
}
.features {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 12px;
}
.tag {
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--vp-c-default-soft);
  color: var(--vp-c-text-2);
  font-size: 12px;
}
.info pre {
  margin: 0;
  padding: 12px;
  overflow-x: auto;
  border-radius: 8px;
  background: var(--vp-code-block-bg);
  font-size: 13px;
}
.copy {
  margin-top: 8px;
  padding: 4px 12px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  font-size: 13px;
}
@media (max-width: 960px) {
  .gallery {
    grid-template-columns: minmax(0, 1fr);
    gap: 16px;
  }
  /* The list becomes a strip to scroll sideways, above the player. */
  .list {
    position: static;
    max-height: none;
    flex-direction: row;
    overflow-x: auto;
    padding-bottom: 8px;
  }
  .group {
    display: none;
  }
  .item {
    flex: none;
    border: 1px solid var(--vp-c-divider);
  }
}
@media (max-width: 1100px) {
  .body {
    grid-template-columns: minmax(0, 1fr);
  }
}
@media (max-width: 480px) {
  .stage {
    padding: 12px;
  }
}
</style>
