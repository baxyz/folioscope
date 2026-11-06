<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  A flip book: screens drawn as simple wireframes, turned one after another. Decorative: the text
  next to it says the same. With reduced motion, the book rests on its first screen.

  How it is built, so it does not jump:
  - the pages are stacked by z-index, not by depth: planes that cross in 3D flicker where they meet,
    and the first page crossed the second as it started to turn. Each page changes its z-index as it
    passes the spine, so a page that lands on the left lies on top of the ones already there;
  - the book ends by fading out and in, while the pages go back to the right unseen, instead of
    turning back under the pile. The first frame, the rest state and the last frame are the same.
-->
<template>
  <div class="flipbook" aria-hidden="true">
    <div class="book">
      <div
        v-for="(screen, i) in screens"
        :key="i"
        class="page"
        :class="{ last: i === screens.length - 1 }"
        :style="{ '--i': i, '--n': screens.length }"
      >
        <div class="face front">
          <span class="bar"></span>
          <template v-if="screen === 'list'">
            <span v-for="n in 3" :key="n" class="row"></span>
          </template>
          <template v-else-if="screen === 'detail'">
            <span class="hero"></span>
            <span class="line"></span>
            <span class="line short"></span>
          </template>
          <template v-else>
            <span class="title"></span>
            <span class="button"></span>
          </template>
          <span class="dot"></span>
        </div>
        <div class="face back">
          <span class="bar"></span>
          <span class="line"></span>
          <span class="line short"></span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const screens = ["home", "list", "detail", "home"];
</script>

<style scoped>
.flipbook {
  flex: none;
  width: 340px;
  height: 190px;
  perspective: 900px;
  margin: 8px auto;
}
/* Flat on purpose (no `preserve-3d`): its own perspective turns the pages, the outer one tilts it. */
.book {
  --period: 9s;
  position: relative;
  width: 100%;
  height: 100%;
  perspective: 900px;
  transform: rotateX(8deg) rotateY(-8deg);
  animation: fade var(--period) ease-in-out infinite;
}
.page {
  /* Where the page stacks while it lies on the right, then on the left. */
  --right: calc(var(--n) - var(--i));
  --left: calc(var(--i) + 1);
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 168px;
  transform-origin: left center;
  transform-style: preserve-3d;
  z-index: var(--right);
  animation: turn var(--period) ease-in-out infinite;
  animation-delay: calc(var(--i) * 0.45s);
}
.face {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  box-sizing: border-box;
  border-radius: 6px;
  backface-visibility: hidden;
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-divider);
  box-shadow: 0 6px 18px rgb(0 0 0 / 0.14);
}
.back {
  transform: rotateY(180deg);
  background: var(--vp-c-bg-soft);
}
/* The last screen stays where it is: the others turn over it. */
.page.last {
  animation: none;
}
.bar,
.row,
.line,
.title {
  display: block;
  height: 10px;
  border-radius: 5px;
  background: var(--vp-c-default-soft);
}
.bar {
  width: 45%;
  background: var(--vp-c-brand-soft);
}
.row {
  height: 22px;
  border-radius: 6px;
}
.hero {
  display: block;
  height: 56px;
  border-radius: 6px;
  background: var(--vp-c-brand-soft);
}
.line.short {
  width: 60%;
}
.title {
  height: 16px;
  width: 70%;
  margin-top: 18px;
}
.button {
  display: block;
  height: 26px;
  width: 50%;
  margin-top: auto;
  border-radius: 13px;
  background: var(--vp-c-brand-1);
}
.dot {
  position: absolute;
  right: 12px;
  bottom: 12px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--vp-c-brand-1);
  opacity: 0.35;
}

/* Each page waits, turns over, stays turned, then goes back to the right while the book is faded. */
@keyframes turn {
  0%,
  12% {
    transform: rotateY(0deg);
    z-index: var(--right);
  }
  /* The page is edge-on: it changes sides, so it lies over the pages already on the left. */
  20% {
    z-index: var(--right);
  }
  20.1% {
    z-index: var(--left);
  }
  28%,
  90% {
    transform: rotateY(-172deg);
    z-index: var(--left);
  }
  90.1%,
  100% {
    transform: rotateY(0deg);
    z-index: var(--right);
  }
}

/* The pages are staggered by up to 0.9 s: the book is hidden from 8.0 s to 9.0 s of the cycle,
   when they go back (8.1 s, 8.55 s, 9.0 s), and shows again as the first page waits. */
@keyframes fade {
  0% {
    opacity: 0;
  }
  4%,
  83% {
    opacity: 1;
  }
  89%,
  100% {
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .book,
  .page {
    animation: none;
  }
}
</style>
