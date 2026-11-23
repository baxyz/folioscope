// SPDX-License-Identifier: AGPL-3.0-or-later
import { describe, expect, test } from "vite-plus/test";
import { linkTarget } from "../src/link.ts";

const base = "https://example.org/portfolio/";

describe("open link", () => {
  test("follows web, e-mail and phone links", () => {
    expect(linkTarget("https://folioscope.baxyz.dev", base)?.href).toBe(
      "https://folioscope.baxyz.dev/",
    );
    expect(linkTarget("http://example.com/a?b#c", base)?.href).toBe("http://example.com/a?b#c");
    expect(linkTarget("mailto:hello@example.org", base)?.protocol).toBe("mailto:");
    expect(linkTarget("tel:+33123456789", base)?.protocol).toBe("tel:");
  });

  test("resolves a link with no scheme against the page", () => {
    expect(linkTarget("case-study", base)?.href).toBe("https://example.org/portfolio/case-study");
  });

  test("never follows a link that would run the design file's code in the page", () => {
    for (const url of [
      "javascript:alert(document.domain)",
      "JavaScript:alert(1)",
      "  javascript:alert(1)",
      "java\tscript:alert(1)",
      "java\nscript:alert(1)",
      "\u0001javascript:alert(1)",
      "data:text/html,<script>alert(1)</script>",
      "vbscript:msgbox(1)",
      "blob:https://example.org/0d5f3a0e",
      "file:///etc/passwd",
      "http://[",
    ]) {
      expect(linkTarget(url, base), url).toBeUndefined();
    }
  });
});
