// SPDX-License-Identifier: AGPL-3.0-or-later

//! Importers and intermediate model (IR) for interactive `.fig`, `.sketch`, `.xd` and `.penpot`
//! prototypes. See `PROJECT.md` at the repository root.

pub mod fig;
pub mod kiwi;

/// Version of the IR schema, written into every baked `.folio` so players can refuse
/// incompatible files.
pub const IR_VERSION: u32 = 0;
