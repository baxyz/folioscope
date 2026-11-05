<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<template>
  <aside class="early-days" role="note">
    <p>
      <strong>Early days.</strong> Folioscope is under development: <code>.fig</code> prototypes
      bake and play today, with click, timed, hover and press links, overlays, scrolling and
      interactive components; <code>.sketch</code>, <code>.penpot</code> and <code>.xd</code>
      prototypes too, with their links and transitions. See what each format supports on the
      <a href="/compatibility">compatibility page</a>, and the
      <a href="https://github.com/baxyz/folioscope/blob/main/PROJECT.md">roadmap</a>.
    </p>
  </aside>
</template>

<style scoped>
.early-days {
  margin: 0 auto;
  padding: 12px 24px;
  max-width: 1152px;
  box-sizing: border-box;
}
.early-days p {
  margin: 0;
  padding: 12px 16px;
  border: 1px solid var(--vp-custom-block-warning-border);
  border-radius: 8px;
  background: var(--vp-custom-block-warning-bg);
  color: var(--vp-custom-block-warning-text);
  font-size: 14px;
  line-height: 24px;
}
.early-days a {
  color: inherit;
  text-decoration: underline;
}
</style>
