# Changelog

All notable changes to this project are documented in this file.

## [0.0.3] - 2026-10-27

### Bug fixes
- **bake**: 🐛 cut truncated text, and size a component to its instance


### Documentation
- **site**: 📝 say up front that folioscope plays prototypes, not design files
- **docs**: 📝 the components fixture plays in figma
- **ai**: 📝 explain the release flow to agents


### Features
- **bake**: ✨ draw instances with their overrides, and play interactive components
- **bake**: ✨ read fonts and lay text out, carry overrides across variants
- **cli**: ✨ give the bake fonts, and list a folder's fonts
- **core**: ✨ convert .sketch files to the IR, and bake any format
- **bake**: ✨ draw and bake .sketch prototypes
- **core**: ✨ convert .penpot files to the IR
- **bake**: ✨ draw and bake .penpot prototypes
- **player**: ✨ play springs and back curves as Figma does
- **player**: ✨ play Figma's smart animate


### Refactoring
- **deps**: ♻️ read binary and transform geometry with helpers4 0.0.9
- **player**: ♻️ use helpers4 3.1.9 in the player, and publish it minified
