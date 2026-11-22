<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- The home page's hero picture: a prototype that plays, in a phone. -->
<template>
  <div class="hero-player">
    <folioscope-player
      src="/folio/smart-animate.folio"
      poster="/folio/smart-animate.folio/poster.webp"
      aria-label="A prototype: open the trip, then come back"
    ></folioscope-player>
    <p class="hint">Try it: tap “Open the trip →”.</p>
  </div>
</template>

<style scoped>
.hero-player {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}
.hero-player folioscope-player {
  display: block;
  width: 240px;
  border: 8px solid #111827;
  border-radius: 36px;
  overflow: hidden;
  box-shadow: 0 24px 60px rgb(37 99 235 / 0.25);
  background: #fff;
}
.hint {
  margin: 0;
  font-size: 13px;
  color: var(--vp-c-text-2);
}
@media (max-width: 959px) {
  .hero-player folioscope-player {
    width: 220px;
  }
}
</style>
