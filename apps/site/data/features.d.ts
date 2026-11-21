export type Format = "fig" | "sketch" | "penpot" | "xd";
export declare const FORMATS: {
  id: Format;
  name: string;
}[];
export type Status =
  /** Tested, and checked in the real design tool or on a real file. */
  | {
      status: "works";
      test: string;
      note?: string;
    }
  /** Tested on files we generated, not yet checked in the real tool or on a real file. */
  | {
      status: "unverified";
      test: string;
      missing: string;
      note?: string;
    }
  /** Not done: `missing` says what it needs (often a file to test it with). */
  | {
      status: "todo";
      missing: string;
    }
  /** The design tool has no such feature. */
  | {
      status: "na";
    };
export interface Feature {
  id: string;
  group: string;
  name: string;
  /** Per format; or `all` for what does not depend on the format. */
  formats:
    | Partial<Record<Format, Status>>
    | {
        all: Status;
      };
}
export declare const FEATURES: Feature[];
