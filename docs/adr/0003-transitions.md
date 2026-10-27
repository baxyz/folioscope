# ADR 0003 — Transitions

- **Status:** Accepted
- **Date:** 2026-10-27
- **Decision ID:** D3

## Context

A prototype's transitions must play in the lite player: no WebAssembly, no rendering in the
browser, only the images baked ahead of time. Most transitions move whole screens (dissolve, slide,
push, move in or out). Figma's **smart animate** is different: layers with the same name on both
screens go from one state to the other (position, size, opacity, colour), the others fade.

## Options

| Option                                           | For                                                      | Against                                                  |
| ------------------------------------------------ | -------------------------------------------------------- | -------------------------------------------------------- |
| A. Frame by frame: bake every intermediate image | Exact, whatever changes                                  | Dozens of images per transition; fixed frame rate; heavy |
| B. **CSS (Web Animations) on baked images**      | Light, smooth at any frame rate, any easing (`linear()`) | Only what moves, scales and fades; no shape morphing     |

## Decision

**B.** The player animates baked images with the Web Animations API:

- **Whole screens**: dissolve, slide, push, move in and out, from the frame's own layers; the
  easing is CSS (`cubic-bezier`, or `linear()` sampled from a spring with helpers4).
- **Smart animate**, basic version: the bake gives every frame a smart animate leads to or from
  its **parts**: its background, then each direct child as an image of its own, at its bounds,
  with its name. The player pairs the parts of the two frames by name: each pair moves and scales
  from its first bounds to its second while the first image fades into the second (so colours,
  opacity and contents change too); a part on one side only fades out or in; the backgrounds
  cross-fade. Then the incoming frame shows as usual.

## Consequences

- Frames reached by smart animate carry their parts too: more images, fetched with the frame's
  neighbours.
- Pairing is by the names of direct children only; nested layers move with their parent. Rotation
  is not animated; a part keeps its unrotated image.
- Without parts (an older `.folio`, another format), smart animate plays as a dissolve.
- A component's change of state with smart animate fades, as before.
