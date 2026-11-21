# Fixtures

Design files used by the tests, one folder per tool (`figma/`, `sketch/`, `xd/`, `penpot/`).

**Only files we created ourselves go here**, or files shared under a free licence (CC BY 4.0, CC0)
with that licence next to them: everything in this repository is redistributed under the AGPL.
What each fixture must cover is on the site's [Roadmap](https://folioscope.baxyz.dev/roadmap).

## Generated fixtures

`.fig` fixtures are written by `tools/fixtures` (`pnpm --filter @folioscope/fixtures-tools generate`):
edit the generator, not the files.

## Third-party files: `external/`

Real files saved by design tools, from the community or elsewhere, go into `fixtures/external/`
(one folder per format). It is git-ignored: use them locally to validate the importers, never
commit them. Tests that read them skip when they are absent.
