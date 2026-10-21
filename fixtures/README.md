# Fixtures

Design files used by the tests, one folder per tool (`figma/`, `sketch/`, `xd/`, `penpot/`).

**Only files we created ourselves go here** — never a third-party mockup, since everything in this
repository is redistributed under the AGPL. The list of cases to cover is in
[PROJECT.md § 10](../PROJECT.md#10-tests-and-fixtures).

## Generated fixtures

`.fig` fixtures are written by `tools/fixtures` (`pnpm --filter @folioscope/fixtures-tools generate`):
edit the generator, not the files.

## Third-party files: `external/`

Real files saved by design tools, from the community or elsewhere, go into `fixtures/external/`
(one folder per format). It is git-ignored: use them locally to validate the importers, never
commit them. Tests that read them skip when they are absent.
