# ADR 0007 — Interactive component states

- **Status:** Accepted
- **Date:** 2026-11-20
- **Decision ID:** D7

## Context

An interactive component is baked as one image per state (ADR 0004): every variant a "change to"
reaches from the one in the file. Until now, the `.fig` importer kept the first 16 and dropped the
others without a word.

The states of independent components add up. They would multiply once a component inside a state
is interactive too (a parent with N states holding a child with M): N × M images, if each
combination is drawn whole. The question is where to stop, and what happens past that point.

What real files show, in the variant sets of the `.penpot` files of ADR 0006 (Penpot 2 variants):

| File                        | Largest sets: variants, and the properties they vary                                                                        |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Variant examples v1         | 81 (level 5, size 2, image 2, primary 2, secondary 12); 80 (type 8, size 2, theme 5); 52 (state 5, type 2, icon 2, theme 5) |
| [Beta Test] Variants sample | 34 (type 4, size 5, state 8); 19 (level 4, state 2, type 3)                                                                 |
| Prototype examples v1.2     | 11 (an image carousel: image 10, mount 6); then sets of 2 or 3 states                                                       |

A set is large because it crosses design properties (size, theme, type). A prototype changes
**one** of them, the state (hover, pressed, selected…), the others staying put: 2 to 8 states in
these files, 11 for a carousel. None of these files changes a variant from an interaction yet, so
this is the shape of real sets, not a count of reachable states.

## Options

| Unit of the limit           | For                                            | Against                                                 |
| --------------------------- | ---------------------------------------------- | ------------------------------------------------------- |
| A. **States per component** | Simple to explain; the unit designers think in | Says nothing of the frame's total                       |
| B. Images per frame         | Bounds what a frame downloads                  | A busy frame loses states of a component it never shows |
| C. Kilobytes per frame      | The real cost                                  | Unpredictable for the designer; depends on the encoder  |

Past the limit:

| Option                                       | For               | Against                                              |
| -------------------------------------------- | ----------------- | ---------------------------------------------------- |
| D. **Leave the extra states out, and warn**  | Works now         | Those states cannot be reached                       |
| E. A dynamic layer, drawn by the full player | Every state works | Needs Phase 4 (WebAssembly and fonts in the browser) |

## Decision

- **A and D**: at most **16 states per interactive component** by default, the first ones the
  importer finds (the variant in the file first). The others are left out, and the bake **warns**,
  naming the component, its frame and how many states it had. `--max-states <n>` (CLI),
  `maxStates` (`folioscope/vite`) and `BakeOptions::max_states` change the number. The importer
  now hands every reachable state to the bake, which applies the limit.
- 16 is twice the largest set of states seen above, and above the carousel: the limit is a safety
  net, not a constraint on ordinary files.
- **No multiplication for nested components** (when they land, Phase 2): a nested interactive
  component is not drawn into each of its parent's states. It is a **sheet**, one SVG served
  _inline_ in the page, with each state a group the player shows or hides; nested states are
  nested groups. N + M groups instead of N × M images, and no limit needed but the sheet's
  weight. Inline, the sheet can point at its photos by URL (`layers/<hash>.avif`), which an SVG
  in `<img>` cannot (ADR 0006). A component whose look depends on its parent's state (different
  overrides, a clip, a blend mode) still needs one group per combination, and the limit above
  counts them.
- E stays for Phase 4, for what neither images nor a sheet can show.

## Consequences

- A file that hits the limit plays, with some states unreachable, and its bake says which.
- The sheet changes the lite player: it puts SVG in the page (parsed, never as HTML, without
  scripts or `foreignObject`) next to its `<img>` layers. The 15 KB budget applies.
- The limit and the sheet are measured again on the first real files with nested interactive
  components.
