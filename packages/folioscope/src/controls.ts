// SPDX-License-Identifier: AGPL-3.0-or-later
import type { FolioIndex, FolioPage } from "./folio.ts";

export const CONTROLS_STYLE = `
.controls { display: flex; gap: 4px; align-items: center; padding: 4px; font: 13px/1.2 system-ui, sans-serif; background: var(--folioscope-controls-background, #1e1e1e); color: var(--folioscope-controls-color, #eee); }
.controls[hidden] { display: none; }
.controls button, .controls select { font: inherit; color: inherit; background: transparent; border: 1px solid #555; border-radius: 4px; padding: 2px 8px; min-height: 28px; cursor: pointer; }
.controls button:disabled { opacity: .4; cursor: default; }
.controls select[hidden] { display: none; }
.controls .spacer { flex: 1; }
`;

export interface ControlActions {
  restart(): void;
  back(): void;
  page(index: number): void;
  flow(index: number): void;
  fullscreen(): void;
}

/** The control bar: restart, back, page and flow pickers, fullscreen. */
export class Controls {
  readonly element: HTMLElement;
  private readonly backButton: HTMLButtonElement;
  private readonly pages: HTMLSelectElement;
  private readonly flows: HTMLSelectElement;

  constructor(actions: ControlActions) {
    const el = document.createElement("div");
    el.className = "controls";
    el.setAttribute("part", "controls");
    el.setAttribute("role", "toolbar");
    el.setAttribute("aria-label", "Prototype controls");
    el.hidden = true;
    const button = (label: string, text: string, action: () => void) => {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = text;
      b.title = label;
      b.setAttribute("aria-label", label);
      b.addEventListener("click", action);
      return b;
    };
    const select = (label: string, onChange: (index: number) => void) => {
      const s = document.createElement("select");
      s.setAttribute("aria-label", label);
      s.addEventListener("change", () => onChange(s.selectedIndex));
      return s;
    };
    this.backButton = button("Back", "←", () => actions.back());
    this.pages = select("Page", (i) => actions.page(i));
    this.flows = select("Flow", (i) => actions.flow(i));
    const spacer = document.createElement("span");
    spacer.className = "spacer";
    el.append(
      button("Restart", "⟲", () => actions.restart()),
      this.backButton,
      this.pages,
      this.flows,
      spacer,
      button("Fullscreen", "⛶", () => actions.fullscreen()),
    );
    this.element = el;
  }

  /** Fills the pickers for a newly loaded document and its current page. */
  load(index: FolioIndex, page: FolioPage, flow: number): void {
    const playable = index.pages.filter((p) => p.frames.length > 0);
    fill(
      this.pages,
      playable.map((p) => p.name),
      playable.indexOf(page),
    );
    fill(
      this.flows,
      page.flows.map((f) => f.name || "Flow"),
      flow,
    );
  }

  update(canGoBack: boolean): void {
    this.backButton.disabled = !canGoBack;
  }
}

/** Options of a picker, hidden when there is nothing to choose. */
function fill(select: HTMLSelectElement, names: string[], selected: number): void {
  select.replaceChildren(
    ...names.map((name) => {
      const option = document.createElement("option");
      option.textContent = name;
      return option;
    }),
  );
  select.selectedIndex = Math.max(0, selected);
  select.hidden = names.length < 2;
}
