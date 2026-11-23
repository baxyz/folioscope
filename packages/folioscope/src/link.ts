// SPDX-License-Identifier: AGPL-3.0-or-later

/** The schemes a design's "open link" may lead to: web pages, an e-mail, a phone call. */
const SCHEMES = ["http:", "https:", "mailto:", "tel:"];

/**
 * Where an "open link" action leads, resolved against `base`; `undefined` when it is not a link
 * a page may follow. A design file is not trusted with the page it plays in: `javascript:` and
 * `data:` links would run its code there.
 */
export function linkTarget(url: string, base: string): URL | undefined {
  let target: URL;
  try {
    target = new URL(url, base);
  } catch {
    return undefined;
  }
  return SCHEMES.includes(target.protocol) ? target : undefined;
}
