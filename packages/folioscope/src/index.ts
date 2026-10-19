// SPDX-License-Identifier: AGPL-3.0-or-later
import { FolioscopePlayer, TAG_NAME } from "./element.ts";

export { FolioscopePlayer, TAG_NAME };

if (typeof customElements !== "undefined" && !customElements.get(TAG_NAME)) {
  customElements.define(TAG_NAME, FolioscopePlayer);
}

declare global {
  interface HTMLElementTagNameMap {
    "folioscope-player": FolioscopePlayer;
  }
}
