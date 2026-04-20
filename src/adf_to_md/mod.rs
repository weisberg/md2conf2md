//! ADF → Markdown conversion.

pub mod blocks;
pub mod extensions;
pub mod inlines;

use crate::adf::model::Document;
use crate::Error;

/// Convert an ADF [`Document`] to a Markdown string.
pub fn convert(doc: &Document) -> Result<String, Error> {
    let mut out = String::new();
    blocks::write_blocks(&doc.content, &mut out, "");

    // Trim trailing whitespace but keep final newline
    let trimmed = out.trim_end().to_string();
    if trimmed.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{trimmed}\n"))
    }
}
