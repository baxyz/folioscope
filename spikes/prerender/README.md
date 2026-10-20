# Pre-render spike (Phase 0, throwaway)

`triggers.fig` baked by hand into layers: the splash as one image, the home screen as a background
plus a button in three states (default, hover, press). `index.html` plays it with a few lines of
JS: images and DOM zones, no WASM, no fonts.

```sh
cargo run -p folioscope-bake --example prerender_spike -- fixtures/fig/triggers.fig spikes/prerender/out
python3 -m http.server -d spikes/prerender 8000   # then open http://localhost:8000
```

Delete this folder once Phase 1's bake and lite player replace it.
