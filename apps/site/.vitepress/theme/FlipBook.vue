<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!--
  A flip book: screens drawn as simple wireframes, turned one after another. Decorative: the text
  next to it says the same. With reduced motion, the book rests on its first screen.
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
.book {
  position: relative;
  width: 100%;
  height: 100%;
  transform-style: preserve-3d;
  transform: rotateX(8deg) rotateY(-8deg);
}
.page {
  --period: 9s;
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 168px;
  transform-origin: left center;
  transform-style: preserve-3d;
  z-index: calc(var(--n) - var(--i));
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

/* Each page waits, turns over, stays turned, then they all come back. */
@keyframes turn {
  0%,
  12% {
    transform: rotateY(0deg);
  }
  28%,
  80% {
    transform: rotateY(-172deg);
  }
  94%,
  100% {
    transform: rotateY(0deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .page {
    animation: none;
  }
}
</style>
