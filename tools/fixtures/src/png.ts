// SPDX-License-Identifier: AGPL-3.0-or-later
import { crc32, deflateSync } from "node:zlib";

/** A checkerboard PNG of `cells` × `cells` squares of `size` pixels, in two colours. */
export function checkerPng(
  size: number,
  cells: number,
  a: { r: number; g: number; b: number },
  b: { r: number; g: number; b: number },
) {
  const side = size * cells;
  const pixel = (x: number, y: number) =>
    (Math.floor(x / size) + Math.floor(y / size)) % 2 ? b : a;
  return png(side, side, pixel);
}

/** A PNG of a single color: 8-bit RGB, unfiltered rows. */
export function solidPng(
  width: number,
  height: number,
  color: { r: number; g: number; b: number },
) {
  return png(width, height, () => color);
}

/** A PNG of `width` × `height` pixels, each of the colour `pixel` gives. */
function png(
  width: number,
  height: number,
  pixel: (x: number, y: number) => { r: number; g: number; b: number },
) {
  const raw = Buffer.concat(
    Array.from({ length: height }, (_, y) => {
      const row = Buffer.alloc(1 + width * 3);
      for (let x = 0; x < width; x++) {
        const c = pixel(x, y);
        row.set(
          [c.r, c.g, c.b].map((v) => Math.round(v * 255)),
          1 + x * 3,
        );
      }
      return row;
    }),
  );
  const chunk = (type: string, data: Buffer) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(data.length, 0);
    head.write(type, 4, "ascii");
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0);
    return Buffer.concat([head, data, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  return new Uint8Array(
    Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      chunk("IHDR", ihdr),
      chunk("IDAT", deflateSync(raw)),
      chunk("IEND", Buffer.alloc(0)),
    ]),
  );
}
