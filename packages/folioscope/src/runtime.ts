// SPDX-License-Identifier: AGPL-3.0-or-later
import type { Action, FolioFrame, FolioPage, Transition } from "./folio.ts";

/** What the screen should show, and how it got there. */
export interface Step {
  frame: FolioFrame;
  /** Absent on the first screen and on `restart()`. */
  transition?: Transition;
  /** Going back plays the transition in reverse. */
  reverse?: boolean;
}

/**
 * The prototype's state machine, independent of the DOM: the current screen of a page and the
 * history `back` returns through.
 */
// TODO(helpers4): rebuild on the helpers4 `fsm` module once it exists.
export class Runtime {
  readonly page: FolioPage;
  private readonly frames: Map<string, FolioFrame>;
  private history: string[] = [];
  private current: string;
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
    this.current = start;
  }

  get frame(): FolioFrame {
    return this.frames.get(this.current)!;
  }

  get canGoBack(): boolean {
    return this.history.length > 0;
  }

  /** Applies an action; returns the step to show, or `undefined` when the screen stays. */
  run(action: Action): Step | undefined {
    switch (action.type) {
      case "navigate":
        return this.goTo(action.to, action.transition);
      case "back":
        return this.back();
      default:
        // Overlays, scrolling and the rest come with later phases.
        return undefined;
    }
  }

  goTo(frameId: string, transition?: Transition): Step | undefined {
    const frame = this.frames.get(frameId);
    if (!frame || frameId === this.current) return undefined;
    this.history.push(this.current);
    this.current = frameId;
    return { frame, transition };
  }

  back(): Step | undefined {
    const previous = this.history.pop();
    if (previous === undefined) return undefined;
    // The transition that led here, played backwards.
    const transition = this.lastTransitionTo(this.current, previous);
    this.current = previous;
    return { frame: this.frame, transition, reverse: true };
  }

  restart(): Step {
    this.history = [];
    this.current = this.start;
    return { frame: this.frame };
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
