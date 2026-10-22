// SPDX-License-Identifier: AGPL-3.0-or-later
import type { Action, FolioFrame, FolioPage, Point, Rect, Transition } from "./folio.ts";

/** An overlay open over the screen. */
export interface OpenOverlay {
  frame: FolioFrame;
  /** The transition it opened with; closing plays it backwards. */
  transition: Transition;
  /** Where a `manual` overlay goes: the hotspot that opened it, and the offset from it. */
  origin?: Rect;
  offset?: Point;
}

/** What the screen should show, and how it got there. */
export interface Step {
  frame: FolioFrame;
  /** Overlays over the frame, the top one last. */
  overlays: OpenOverlay[];
  /** Absent on the first screen and on `restart()`. */
  transition?: Transition;
  /** Going back plays the transition in reverse. */
  reverse?: boolean;
  /** `overlay` when only the top overlay changed: the screen below stays as it is. */
  layer?: "screen" | "overlay";
}

interface State {
  frame: string;
  overlays: { id: string; transition: Transition; origin?: Rect; offset?: Point }[];
}

/**
 * The prototype's state machine, independent of the DOM: the current screen of a page, the
 * overlays over it, and the history `back` returns through.
 */
// TODO(helpers4): rebuild on the helpers4 `fsm` module once it exists.
export class Runtime {
  readonly page: FolioPage;
  private readonly frames: Map<string, FolioFrame>;
  private history: State[] = [];
  /** Frames left by a hover or press still under way, innermost last; `back` skips them. */
  private held: string[] = [];
  private state: State;
  private readonly start: string;

  /** Starts on `flow` (name or index), else the page's first flow, else its first frame. */
  constructor(page: FolioPage, flow?: string) {
    this.page = page;
    this.frames = new Map(page.frames.map((f) => [f.id, f]));
    const byIndex = flow !== undefined && /^\d+$/.test(flow) ? page.flows[Number(flow)] : undefined;
    const chosen = byIndex ?? page.flows.find((f) => f.name === flow) ?? page.flows[0];
    const start = chosen?.start ?? page.frames[0]?.id;
    if (start === undefined || !this.frames.has(start))
      throw new Error(`page "${page.name}" has no frame to start on`);
    this.start = start;
    this.state = { frame: start, overlays: [] };
  }

  /** The screen below the overlays. */
  get frame(): FolioFrame {
    return this.frames.get(this.state.frame)!;
  }

  /** The frame on top: the last overlay, else the screen. */
  get top(): FolioFrame {
    const overlay = this.state.overlays.at(-1);
    return overlay ? this.frames.get(overlay.id)! : this.frame;
  }

  /** How many hovers and presses currently hold a frame. */
  get holding(): number {
    return this.held.length;
  }

  get canGoBack(): boolean {
    return this.history.length > 0;
  }

  /**
   * Applies an action; returns the step to show, or `undefined` when the screen stays. `origin`
   * is the hotspot that ran it, where a `manual` overlay opens.
   */
  run(action: Action, origin?: Rect): Step | undefined {
    switch (action.type) {
      case "navigate":
        return this.goTo(action.to, action.transition);
      case "overlay":
      case "swapOverlay": {
        if (!this.frames.has(action.to)) return undefined;
        const overlays = [...this.state.overlays];
        if (action.type === "swapOverlay") overlays.pop();
        overlays.push({
          id: action.to,
          transition: action.transition,
          origin,
          offset: action.offset,
        });
        this.commit({ frame: this.base, overlays });
        return this.step({ transition: action.transition, layer: "overlay" });
      }
      case "close":
        return this.close();
      case "back":
        return this.back();
      default:
        // Scrolling and the rest come with later phases.
        return undefined;
    }
  }

  goTo(frameId: string, transition?: Transition): Step | undefined {
    if (!this.frames.has(frameId) || (frameId === this.state.frame && !this.state.overlays.length))
      return undefined;
    this.commit({ frame: frameId, overlays: [] });
    return this.step({ transition });
  }

  /**
   * "While hovering" and "while pressing": shows the action's frame until `release()`, without
   * adding it to the history. Not inside overlays yet.
   */
  hold(action: Action): Step | undefined {
    if (action.type !== "navigate" || this.state.overlays.length > 0) return undefined;
    if (!this.frames.has(action.to) || action.to === this.state.frame) return undefined;
    this.held.push(this.state.frame);
    this.state = { frame: action.to, overlays: [] };
    return this.step({ transition: action.transition });
  }

  /** Ends the innermost hold: back to the frame it started from, at once. */
  release(): Step | undefined {
    const from = this.held.pop();
    if (from === undefined) return undefined;
    this.state = { frame: from, overlays: [] };
    return this.step();
  }

  /** Closes the top overlay, with its opening transition played backwards. */
  close(): Step | undefined {
    const top = this.state.overlays.at(-1);
    if (!top) return undefined;
    const below = { frame: this.state.frame, overlays: this.state.overlays.slice(0, -1) };
    // Closing what was just opened takes it out of the history too.
    if (same(this.history.at(-1), below)) this.history.pop();
    this.state = below;
    return this.step({ transition: top.transition, reverse: true, layer: "overlay" });
  }

  back(): Step | undefined {
    // Back from a held frame starts from the frame the hold left.
    const base = this.held[0];
    this.held = [];
    if (base !== undefined) this.state = { frame: base, overlays: [] };
    const previous = this.history.pop();
    if (previous === undefined) return base === undefined ? undefined : this.step();
    const leaving = this.state;
    this.state = previous;
    if (previous.frame === leaving.frame) {
      // Only overlays changed: undo the top one.
      const top = leaving.overlays.at(-1) ?? previous.overlays.at(-1);
      return this.step({ transition: top?.transition, reverse: true, layer: "overlay" });
    }
    // The transition that led here, played backwards.
    const transition = this.lastTransitionTo(leaving.frame, previous.frame);
    return this.step({ transition, reverse: true });
  }

  restart(): Step {
    this.history = [];
    this.held = [];
    this.state = { frame: this.start, overlays: [] };
    return this.step();
  }

  /** The frame below any hold. */
  private get base(): string {
    return this.held[0] ?? this.state.frame;
  }

  /** Moves to `next`, recording where it came from; a hold under way is committed. */
  private commit(next: State): void {
    this.history.push(this.held.length ? { frame: this.held[0]!, overlays: [] } : this.state);
    this.held = [];
    this.state = next;
  }

  private step(how: Omit<Step, "frame" | "overlays"> = {}): Step {
    return {
      frame: this.frame,
      overlays: this.state.overlays.map(({ id, ...rest }) => ({
        frame: this.frames.get(id)!,
        ...rest,
      })),
      ...how,
    };
  }

  private lastTransitionTo(to: string, from: string): Transition | undefined {
    for (const hotspot of this.frames.get(from)?.hotspots ?? []) {
      for (const { action } of hotspot.interactions) {
        if (action.type === "navigate" && action.to === to) return action.transition;
      }
    }
    return undefined;
  }
}

function same(a: State | undefined, b: State): boolean {
  return (
    a !== undefined &&
    a.frame === b.frame &&
    a.overlays.length === b.overlays.length &&
    a.overlays.every((o, i) => o.id === b.overlays[i]!.id)
  );
}
