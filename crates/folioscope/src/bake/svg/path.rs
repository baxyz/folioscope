// SPDX-License-Identifier: AGPL-3.0-or-later

//! Figma's path command blobs, as SVG path data.

use std::fmt::Write;

/// Appends the SVG path data of a geometry blob: a command byte (0 close, 1 move, 2 line,
/// 3 quadratic, 4 cubic) followed by little-endian `f32` points. Stops at an unknown command.
/// Glyph outlines use quadratic curves, as TrueType fonts do.
pub(super) fn path_commands(bytes: &[u8], d: &mut String, map: impl Fn(f32, f32) -> (f32, f32)) {
    let mut i = 0;
    let point = |i: &mut usize| -> Option<(f32, f32)> {
        let x = f32::from_le_bytes(bytes.get(*i..*i + 4)?.try_into().ok()?);
        let y = f32::from_le_bytes(bytes.get(*i + 4..*i + 8)?.try_into().ok()?);
        *i += 8;
        Some(map(x, y))
    };
    // Glyph outlines start with a close command; SVG rejects a path that does.
    let mut open = false;
    while let Some(&cmd) = bytes.get(i) {
        i += 1;
        let ok = match cmd {
            0 => {
                if open {
                    d.push('Z');
                    open = false;
                }
                Some(())
            }
            1 | 2 => point(&mut i).map(|(x, y)| {
                let _ = write!(d, "{}{x} {y}", if cmd == 1 { 'M' } else { 'L' });
                open = true;
            }),
            3 => (|| {
                let (x1, y1) = point(&mut i)?;
                let (x, y) = point(&mut i)?;
                let _ = write!(d, "Q{x1} {y1} {x} {y}");
                Some(())
            })(),
            4 => (|| {
                let (x1, y1) = point(&mut i)?;
                let (x2, y2) = point(&mut i)?;
                let (x, y) = point(&mut i)?;
                let _ = write!(d, "C{x1} {y1} {x2} {y2} {x} {y}");
                Some(())
            })(),
            _ => None,
        };
        if ok.is_none() {
            return;
        }
    }
}
