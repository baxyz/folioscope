// SPDX-License-Identifier: AGPL-3.0-or-later

export const TAG_NAME = "folioscope-player";

// Keeps the module importable during SSR (VitePress build, the Vite plugin), where HTMLElement is undefined.
const BaseElement: typeof HTMLElement =
  globalThis.HTMLElement ?? (class {} as unknown as typeof HTMLElement);

export class FolioscopePlayer extends BaseElement {
  static readonly observedAttributes: readonly string[] = ["src"];

  get src(): string {
    return this.getAttribute("src") ?? "";
  }

  set src(value: string) {
    this.setAttribute("src", value);
  }
}
