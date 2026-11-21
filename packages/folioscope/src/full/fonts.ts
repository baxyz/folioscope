// SPDX-License-Identifier: AGPL-3.0-or-later
// Which fonts the full player fetches: the page's declarations, matched against the families the
// design's text uses; `@font-face` rules read from a stylesheet.
import { parsePatternOrRegExp } from "helpers4/string";
import type { FontDeclaration } from "./index.ts";

/**
 * Whether a declared font may serve the design: a font with no family nor rule (its file says
 * what it is) or bytes already given, a family the design uses, or a rule matching one.
 */
export function needed(font: FontDeclaration, used: string[]): boolean {
  if (font.data || (!font.family && !font.match?.length)) return true;
  const lower = used.map((family) => family.toLowerCase());
  if (font.family && lower.includes(font.family.toLowerCase())) return true;
  return (font.match ?? []).some((rule) => {
    try {
      const pattern = parsePatternOrRegExp(rule, { caseInsensitive: true });
      return used.some((family) => pattern.test(family));
    } catch {
      return false;
    }
  });
}

/** The `@font-face` rules of a stylesheet, as declarations; `src` URLs resolved against `base`. */
export function fontFaces(sheet: string, base: string): FontDeclaration[] {
  const out: FontDeclaration[] = [];
  for (const [, body] of sheet.matchAll(/@font-face\s*\{([^}]*)\}/g)) {
    const property = (name: string) =>
      new RegExp(`(?:^|;)\\s*${name}\\s*:\\s*([^;]+)`, "i").exec(body!)?.[1]?.trim();
    const family = property("font-family")?.replace(/^["']|["']$/g, "");
    // The first `url()` that is not a local font; WOFF2 first when several are given.
    const urls = [...(property("src") ?? "").matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g)].map(
      (m) => m[1]!,
    );
    const src = urls.find((u) => /\.woff2(\?|#|$)/i.test(u)) ?? urls[0];
    if (!family || !src) continue;
    const weight = property("font-weight");
    out.push({
      url: new URL(src, base).href,
      family,
      weight: weight && /^\d+$/.test(weight) ? Number(weight) : undefined,
      italic: /^(italic|oblique)/i.test(property("font-style") ?? "") || undefined,
    });
  }
  return out;
}
