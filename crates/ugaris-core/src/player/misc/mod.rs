// The PPD byte-offset constants and codecs in this module mirror the C
// `struct *_ppd` layouts verbatim as `<field index> * 4` products (so
// `0 * 4`, `1 * 4`, ... line up visually with the C struct order); keep
// clippy from "simplifying" the intentional identity/zero terms.
#![allow(clippy::identity_op, clippy::erasing_op)]

mod ppd_codec;
mod ppd_consts;
mod ppd_offsets;
mod types;

pub use ppd_consts::*;
pub use ppd_offsets::*;
pub use types::*;

use super::*;
