// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test } from "vite-plus/test";

test("importing the package outside a browser does not throw", async () => {
  const mod = await import("../src/index.ts").catch((error: unknown) => error);
  expect(mod).not.toBeInstanceOf(Error);
});
