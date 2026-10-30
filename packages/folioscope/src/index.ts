// SPDX-License-Identifier: AGPL-3.0-or-later
// The entry of the npm package: the player, plus the typing of its tag. JSR does not accept global
// augmentations, so jsr.json points at player.ts instead.
import type { FolioscopePlayer } from "./player.ts";

export * from "./player.ts";

declare global {
  interface HTMLElementTagNameMap {
    "folioscope-player": FolioscopePlayer;
  }
}
