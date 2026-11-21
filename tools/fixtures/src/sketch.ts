// SPDX-License-Identifier: AGPL-3.0-or-later
// Types only: the package entry re-exports them as a CommonJS default, which `nodenext` hides.
import type * as FileFormat from "@sketch-hq/sketch-file-format-ts/dist/cjs/types.js";
import { zipSync } from "fflate";
import type { Box } from "./figma.ts";
import { solidPng } from "./png.ts";

const FIXED_DATE = new Date("2026-01-01T00:00:00Z");

export type SketchAnimation =
  | "none"
  | "slideFromLeft"
  | "slideFromRight"
  | "slideFromBottom"
  | "slideFromTop";

// Real files write -1 for "no animation", though schema 6.5 starts its enum at 0.
const ANIMATIONS: Record<SketchAnimation, number> = {
  none: -1,
  slideFromLeft: 1,
  slideFromRight: 2,
  slideFromBottom: 3,
  slideFromTop: 4,
};

type Layer = FileFormat.Rectangle | FileFormat.Hotspot | FileFormat.Oval | FileFormat.Bitmap;
type Parent = FileFormat.Artboard | FileFormat.Group;

/**
 * Builds a `.sketch` file, typed against Sketch's published file format. Ids are deterministic so
 * regenerating is a no-op.
 */
export class SketchBuilder {
  private nextId = 1;
  private readonly pages: FileFormat.Page[] = [];
  private readonly images = new Map<string, Uint8Array>();

  /** UUID-shaped and deterministic; uppercase, as Sketch writes them. */
  private id(): string {
    return `00000000-0000-4000-8000-${(this.nextId++).toString(16).padStart(12, "0")}`.toUpperCase();
  }

  page(name: string): FileFormat.Page {
    const page: FileFormat.Page = {
      ...this.base(name, { x: 0, y: 0, width: 0, height: 0 }),
      _class: "page",
      ...this.container(),
      hasClickThrough: true,
      horizontalRulerData: { _class: "rulerData", base: 0, guides: [] },
      verticalRulerData: { _class: "rulerData", base: 0, guides: [] },
      layers: [],
    };
    this.pages.push(page);
    return page;
  }

  artboard(
    page: FileFormat.Page,
    name: string,
    box: Box,
    background = "#FFFFFF",
  ): FileFormat.Artboard {
    const artboard: FileFormat.Artboard = {
      ...this.base(name, box),
      _class: "artboard",
      ...this.container(),
      grid: { _class: "simpleGrid", isEnabled: false, gridSize: 8, thickGridTimes: 10 },
      hasClickThrough: false,
      horizontalRulerData: { _class: "rulerData", base: 0, guides: [] },
      verticalRulerData: { _class: "rulerData", base: 0, guides: [] },
      backgroundColor: color(background),
      hasBackgroundColor: true,
      includeBackgroundColorInExport: true,
      isFlowHome: false,
      resizesContent: false,
      layers: [],
    };
    page.layers.push(artboard);
    // Converters write the fill as a first, full-size rectangle; Lunacy plays such files.
    this.rect(
      artboard,
      `${name} (Background)`,
      { x: 0, y: 0, width: box.width, height: box.height },
      background,
    );
    return artboard;
  }

  rect(artboard: Parent, name: string, box: Box, fill: string, radius = 0): FileFormat.Rectangle {
    const corner = (point: string): FileFormat.CurvePoint => ({
      _class: "curvePoint",
      cornerRadius: radius,
      cornerStyle: 0,
      curveFrom: point,
      curveTo: point,
      hasCurveFrom: false,
      hasCurveTo: false,
      curveMode: 1,
      point,
    });
    const rect: FileFormat.Rectangle = {
      ...this.base(name, box),
      _class: "rectangle",
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(fill),
      edited: false,
      isClosed: true,
      pointRadiusBehaviour: 1,
      points: ["{0, 0}", "{1, 0}", "{1, 1}", "{0, 1}"].map(corner),
      // The radius lives on the points; Sketch keeps fixedRadius for legacy shapes.
      fixedRadius: 0,
      hasConvertedToNewRoundCorners: true,
      needsConvertionToNewRoundCorners: false,
    };
    artboard.layers.push(rect);
    return rect;
  }

  /**
   * A text layer in one font, named by its PostScript name as Sketch does (`Inter-Regular`). The
   * glyphs are not stored: whoever draws it needs the font.
   */
  text(
    artboard: FileFormat.Artboard,
    name: string,
    box: Box,
    string: string,
    font: { name: string; size: number },
    fill = "#111827",
  ): FileFormat.Text {
    const descriptor: FileFormat.FontDescriptor = {
      _class: "fontDescriptor",
      attributes: { name: font.name, size: font.size },
    };
    const text: FileFormat.Text = {
      ...this.base(name, box),
      _class: "text",
      style: {
        ...this.style(),
        textStyle: {
          _class: "textStyle",
          verticalAlignment: 0 as FileFormat.TextVerticalAlignment, // top,
          encodedAttributes: {
            MSAttributedStringFontAttribute: descriptor,
            MSAttributedStringColorAttribute: color(fill),
          },
        },
      },
      attributedString: {
        _class: "attributedString",
        string,
        attributes: [
          {
            _class: "stringAttribute",
            location: 0,
            length: string.length,
            attributes: {
              MSAttributedStringFontAttribute: descriptor,
              MSAttributedStringColorAttribute: color(fill),
              paragraphStyle: { _class: "paragraphStyle", alignment: 0 },
              kerning: 0,
            },
          },
        ],
      },
      automaticallyDrawOnUnderlyingPath: false,
      dontSynchroniseWithSymbol: false,
      lineSpacingBehaviour: 1 as FileFormat.LineSpacingBehaviour, // legacy
      // Grows with its content: no wrapping.
      textBehaviour: 0 as FileFormat.TextBehaviour, // flexible
      glyphBounds: "{{0, 0}, {0, 0}}",
    };
    artboard.layers.push(text);
    return text;
  }

  /** An ellipse filling its box. */
  oval(parent: Parent, name: string, box: Box, fill: string): FileFormat.Oval {
    const point = (p: string, from: string, to: string): FileFormat.CurvePoint => ({
      _class: "curvePoint",
      cornerRadius: 0,
      cornerStyle: 0,
      curveFrom: from,
      curveTo: to,
      hasCurveFrom: true,
      hasCurveTo: true,
      curveMode: 2,
      point: p,
    });
    // The Bézier handle length of a quarter circle, as a share of the diameter.
    const k = 0.27614237490000004;
    const oval: FileFormat.Oval = {
      ...this.base(name, box),
      _class: "oval",
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(fill),
      edited: false,
      isClosed: true,
      pointRadiusBehaviour: 1,
      points: [
        point("{0.5, 1}", `{${0.5 + k * 1.0}, 1}`, `{${0.5 - k * 1.0}, 1}`),
        point("{1, 0.5}", `{1, ${0.5 - k}}`, `{1, ${0.5 + k}}`),
        point("{0.5, 0}", `{${0.5 - k}, 0}`, `{${0.5 + k}, 0}`),
        point("{0, 0.5}", `{0, ${0.5 + k}}`, `{0, ${0.5 - k}}`),
      ],
    };
    parent.layers.push(oval);
    return oval;
  }

  /** A bitmap of one colour, stored in the file's images/. */
  bitmap(
    parent: Parent,
    name: string,
    box: Box,
    rgb: { r: number; g: number; b: number },
  ): FileFormat.Bitmap {
    const ref = `images/${this.id().toLowerCase()}.png`;
    this.images.set(ref, solidPng(8, 8, rgb));
    const bitmap = {
      ...this.base(name, box),
      _class: "bitmap",
      clippingMask: "{{0, 0}, {1, 1}}",
      fillReplacesImage: false,
      intendedDPI: 72,
      image: { _class: "MSJSONFileReference", _ref_class: "MSImageData", _ref: ref },
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(),
    } as unknown as FileFormat.Bitmap;
    parent.layers.push(bitmap);
    return bitmap;
  }

  /** A group of layers, at `box` inside its parent. */
  group(parent: Parent, name: string, box: Box): FileFormat.Group {
    const group: FileFormat.Group = {
      ...this.base(name, box),
      _class: "group",
      ...this.container(),
      hasClickThrough: false,
      layers: [],
    };
    parent.layers.push(group);
    return group;
  }

  /** Makes `layer` a mask: it clips the layers after it in its group. */
  mask(layer: Layer): void {
    (layer as FileFormat.Rectangle).hasClippingMask = true;
  }

  /** Replaces a layer's first fill with a linear (0), radial (1) or angular (2) gradient. */
  gradient(layer: Layer, type: 0 | 1 | 2, from: string, to: string): void {
    const fill = (layer as FileFormat.Rectangle).style!.fills![0]!;
    fill.fillType = 1 as FileFormat.FillType;
    fill.gradient = {
      _class: "gradient",
      gradientType: type as FileFormat.GradientType,
      elipseLength: 1,
      from: type === 1 ? "{0.5, 0.5}" : "{0, 0.5}",
      to: type === 1 ? "{1, 0.5}" : "{1, 0.5}",
      stops: [
        { _class: "gradientStop", position: 0, color: color(from) },
        { _class: "gradientStop", position: 1, color: color(to) },
      ],
    };
  }

  /** Replaces a layer's first fill with an image: tiled (0, at `scale`), fill (1), stretch (2), fit (3). */
  imageFill(layer: Layer, png: Uint8Array, mode: 0 | 1 | 2 | 3, scale = 1): void {
    const ref = `images/${this.id().toLowerCase()}.png`;
    this.images.set(ref, png);
    const fill = (layer as FileFormat.Rectangle).style!.fills![0]!;
    fill.fillType = 4 as FileFormat.FillType;
    fill.patternFillType = mode as FileFormat.PatternFillType;
    fill.patternTileScale = scale;
    (fill as unknown as { image: object }).image = {
      _class: "MSJSONFileReference",
      _ref_class: "MSImageData",
      _ref: ref,
    };
  }

  /** Adds an inner shadow. */
  innerShadow(layer: Layer, offsetY: number, blur: number, hex: string): void {
    (layer as FileFormat.Rectangle).style!.innerShadows!.push({
      _class: "innerShadow",
      isEnabled: true,
      blurRadius: blur,
      color: color(hex),
      contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
      offsetX: 0,
      offsetY,
      spread: 0,
    });
  }

  /** A blur: 0 Gaussian (the layer), 3 background. */
  blur(layer: Layer, type: 0 | 3, radius: number): void {
    const style = (layer as FileFormat.Rectangle).style!;
    style.blur = { ...style.blur!, isEnabled: true, type: type as FileFormat.BlurType, radius };
  }

  /** A blend mode, as Sketch numbers them (2 multiply). */
  blend(layer: Layer, mode: number): void {
    (layer as FileFormat.Rectangle).style!.contextSettings!.blendMode =
      mode as FileFormat.BlendMode;
  }

  /** A border: position 0 centre, 1 inside, 2 outside. */
  border(layer: Layer, hex: string, thickness: number, position: 0 | 1 | 2): void {
    (layer as FileFormat.Rectangle).style!.borders!.push({
      _class: "border",
      isEnabled: true,
      color: color(hex),
      fillType: 0 as FileFormat.FillType,
      position: position as FileFormat.BorderPosition,
      thickness,
      contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
      gradient: {
        _class: "gradient",
        gradientType: 0,
        elipseLength: 0,
        from: "{0.5, 0}",
        to: "{0.5, 1}",
        stops: [],
      },
    });
  }

  /** The device the artboard is prototyped on: a viewport smaller than it makes it scroll. */
  viewport(artboard: FileFormat.Artboard, name: string, width: number, height: number): void {
    artboard.prototypeViewport = {
      _class: "MSImmutablePrototypeViewport",
      name,
      size: `{${width}, ${height}}`,
    };
  }

  /** Keeps a layer in place while its artboard scrolls. */
  fixed(layer: Layer): void {
    (layer as FileFormat.Rectangle).isFixedToViewport = true;
  }

  /** Makes `layer` an alpha mask: its transparency masks the layers after it in its group. */
  alphaMask(layer: Layer): void {
    (layer as FileFormat.Rectangle).hasClippingMask = true;
    (layer as FileFormat.Rectangle).clippingMaskMode = 1;
  }

  /** Adds a drop shadow. */
  shadow(layer: Layer, offsetY: number, blur: number, hex: string): void {
    (layer as FileFormat.Rectangle).style!.shadows!.push({
      _class: "shadow",
      isEnabled: true,
      blurRadius: blur,
      color: color(hex),
      contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
      offsetX: 0,
      offsetY,
      spread: 0,
    });
  }

  /**
   * A text of several runs, each in its own font, colour and decorations; `width` makes it wrap.
   */
  richText(
    artboard: FileFormat.Artboard,
    name: string,
    box: Box,
    runs: {
      text: string;
      font: string;
      size: number;
      fill: string;
      underline?: boolean;
      strike?: boolean;
    }[],
  ): FileFormat.Text {
    const text = this.text(artboard, name, box, runs.map((r) => r.text).join(""), {
      name: runs[0]!.font,
      size: runs[0]!.size,
    });
    let location = 0;
    text.attributedString.attributes = runs.map((run) => {
      const attribute: FileFormat.StringAttribute = {
        _class: "stringAttribute",
        location,
        length: run.text.length,
        attributes: {
          MSAttributedStringFontAttribute: {
            _class: "fontDescriptor",
            attributes: { name: run.font, size: run.size },
          },
          MSAttributedStringColorAttribute: color(run.fill),
          paragraphStyle: { _class: "paragraphStyle", alignment: 0 },
          kerning: 0,
          ...(run.underline ? { underlineStyle: 1 } : {}),
          ...(run.strike ? { strikethroughStyle: 1 } : {}),
        },
      };
      location += run.text.length;
      return attribute;
    });
    return text;
  }

  /** An invisible clickable area, Sketch's way to add a link without a visible layer. */
  hotspot(artboard: FileFormat.Artboard, name: string, box: Box): FileFormat.Hotspot {
    const hotspot: FileFormat.Hotspot = {
      ...this.base(name, box),
      _class: "MSImmutableHotspotLayer",
    };
    artboard.layers.push(hotspot);
    return hotspot;
  }

  /** Sketch's only trigger is a click or tap. */
  link(layer: Layer, to: FileFormat.Artboard | "back", animation: SketchAnimation = "none"): void {
    layer.flow = {
      _class: "MSImmutableFlowConnection",
      destinationArtboardID: to === "back" ? "back" : to.do_objectID,
      animationType: ANIMATIONS[animation],
    };
  }

  flowHome(artboard: FileFormat.Artboard): void {
    artboard.isFlowHome = true;
  }

  toBytes(): Uint8Array {
    const json = (value: unknown) => new TextEncoder().encode(JSON.stringify(classFirst(value)));
    const document: FileFormat.Document = {
      _class: "document",
      do_objectID: this.id(),
      assets: {
        _class: "assetCollection",
        do_objectID: this.id(),
        colorAssets: [],
        gradientAssets: [],
        images: [],
        colors: [],
        gradients: [],
        exportPresets: [],
        imageCollection: { _class: "imageCollection", images: {} },
      },
      colorSpace: 1,
      currentPageIndex: 0,
      foreignLayerStyles: [],
      foreignSymbols: [],
      foreignTextStyles: [],
      foreignSwatches: [],
      fontReferences: [],
      layerStyles: { _class: "sharedStyleContainer", do_objectID: this.id(), objects: [] },
      layerSymbols: { _class: "symbolContainer", do_objectID: this.id(), objects: [] },
      sharedSwatches: { _class: "swatchContainer", do_objectID: this.id(), objects: [] },
      documentState: { _class: "documentState" },
      layerTextStyles: { _class: "sharedTextStyleContainer", do_objectID: this.id(), objects: [] },
      perDocumentLibraries: [],
      pages: this.pages.map((page) => ({
        _class: "MSJSONFileReference",
        _ref_class: "MSImmutablePage",
        _ref: `pages/${page.do_objectID}`,
      })),
    };
    const created = {
      commit: "folioscope-fixtures",
      appVersion: "69.1",
      build: 0,
      app: "com.bohemiancoding.sketch3" as FileFormat.BundleId,
      compatibilityVersion: 99,
      version: 134,
      variant: "NONAPPSTORE" as FileFormat.SketchVariant,
    };
    const meta: FileFormat.Meta = {
      ...created,
      version: 134,
      compatibilityVersion: 99,
      pagesAndArtboards: Object.fromEntries(
        this.pages.map((page) => [
          page.do_objectID,
          {
            name: page.name,
            artboards: Object.fromEntries(
              page.layers
                .filter((l) => l._class === "artboard")
                .map((a) => [a.do_objectID, { name: a.name }]),
            ),
          },
        ]),
      ),
      autosaved: 0,
      created,
      saveHistory: [],
    };
    Object.assign(meta, { fonts: [] }); // as above
    const user: FileFormat.User = {
      document: { pageListHeight: 110, pageListCollapsed: 0 },
      ...Object.fromEntries(
        this.pages.map((page) => [
          page.do_objectID,
          { scrollOrigin: "{100, 100}", zoomValue: 0.5 },
        ]),
      ),
    };
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    return zipSync({
      "document.json": [json(document), stored],
      "meta.json": [json(meta), stored],
      "user.json": [json(user), stored],
      ...Object.fromEntries(
        this.pages.map((page) => [`pages/${page.do_objectID}.json`, [json(page), stored]]),
      ),
      "previews/preview.png": [solidPng(400, 300, { r: 0.96, g: 0.96, b: 0.96 }), stored],
      ...Object.fromEntries([...this.images].map(([ref, png]) => [ref, [png, stored]])),
    });
  }

  /** The fields every layer shares. */
  private base(name: string, box: Box) {
    return {
      do_objectID: this.id(),
      booleanOperation: -1,
      exportOptions: {
        _class: "exportOptions" as const,
        exportFormats: [],
        includedLayerIds: [],
        layerOptions: 0,
        shouldTrim: false,
      },
      frame: { _class: "rect" as const, constrainProportions: false, ...box },
      isFixedToViewport: false,
      isFlippedHorizontal: false,
      isFlippedVertical: false,
      isLocked: false,
      isTemplate: false,
      isVisible: true,
      layerListExpandedType: 0,
      name,
      nameIsFixed: true,
      resizingConstraint: 63,
      resizingType: 0,
      rotation: 0,
      shouldBreakMaskChain: false,
    };
  }

  /** Fields Sketch writes on pages and artboards, optional in the schema but expected by tools. */
  private container() {
    return {
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(),
      groupLayout: { _class: "MSImmutableFreeformGroupLayout" as const },
      includeInCloudUpload: true,
    };
  }

  private style(fill?: string): FileFormat.Style {
    return {
      _class: "style",
      do_objectID: this.id(),
      borderOptions: {
        _class: "borderOptions",
        isEnabled: true,
        dashPattern: [],
        lineCapStyle: 0,
        lineJoinStyle: 0,
      },
      blur: {
        _class: "blur",
        isEnabled: false,
        center: "{0.5, 0.5}",
        motionAngle: 0,
        radius: 10,
        saturation: 1,
        type: 0,
      },
      borders: [],
      shadows: [],
      contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
      fills:
        fill === undefined
          ? []
          : [
              {
                _class: "fill",
                isEnabled: true,
                color: color(fill),
                fillType: 0,
                noiseIndex: 0,
                noiseIntensity: 0,
                patternFillType: 1,
                patternTileScale: 1,
                contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
                gradient: {
                  _class: "gradient",
                  gradientType: 0,
                  elipseLength: 0,
                  from: "{0.5, 0}",
                  to: "{0.5, 1}",
                  stops: [],
                },
              },
            ],
      startMarkerType: 0,
      endMarkerType: 0,
      miterLimit: 10,
      windingRule: 1,
      innerShadows: [],
      colorControls: {
        _class: "colorControls",
        isEnabled: false,
        brightness: 0,
        contrast: 1,
        hue: 0,
        saturation: 1,
      },
    };
  }
}

function color(hex: string): FileFormat.Color {
  const h = hex.replace("#", "");
  const channel = (i: number) => parseInt(h.slice(i, i + 2), 16) / 255;
  return {
    _class: "color",
    red: channel(0),
    green: channel(2),
    blue: channel(4),
    alpha: h.length >= 8 ? channel(6) : 1,
  };
}

/**
 * Lunacy, a .NET reader, picks each object's type from `_class` and expects it first: with `_class`
 * after other keys, it did not recognize artboards at all. Rebuilds every object with `_class` first.
 */
function classFirst(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(classFirst);
  if (value === null || typeof value !== "object") return value;
  const { _class, ...rest } = value as Record<string, unknown>;
  const entries = Object.entries(rest).map(([k, v]) => [k, classFirst(v)] as const);
  return Object.fromEntries(_class === undefined ? entries : [["_class", _class], ...entries]);
}
