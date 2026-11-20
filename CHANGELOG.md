# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

Flat screens are now served as SVG when that is lighter than their images: sharp at any zoom,
one file for every density (ADR 0006; `--no-svg`, `svg: false`). The states baked per interactive
component are capped at 16 by default, with a warning past it (ADR 0007; `--max-states`,
`maxStates`). Penpot texts whose font family is quoted now find their font.

## [0.0.8] - 2026-10-31

### Documentation

- **ai**: 📝 describe the release of one crate, not three

## [0.0.7] - 2026-10-30

### Bug fixes

- **build**: 🐛 keep the element's registration when player.ts holds it

## [0.0.6] - 2026-10-30

_No notable change._

## [0.0.5] - 2026-10-30

### Documentation

- **docs**: 📝 ask helpers4 for every generic helper Folioscope writes
