//! `md2conf2md` — bidirectional Markdown ↔ Confluence ADF converter.
//!
//! Two entry points:
//!
//! * [`md_to_adf()`] / [`adf_to_md()`] — work with strongly-typed [`Document`] values.
//! * [`md_to_adf_json()`] / [`adf_json_to_md()`] — string-in/string-out, when
//!   you only need the JSON wire format.
//!
//! All ADF node and mark types are re-exported at the crate root so downstream
//! code can pattern-match without reaching into [`adf::model`]:
//!
//! ```
//! use md2conf2md::{Document, Node};
//!
//! let doc: Document = md2conf2md::md_to_adf("# Hello\n\nWorld").unwrap();
//! assert!(matches!(doc.content.first(), Some(Node::Heading { .. })));
//!
//! let md = md2conf2md::adf_to_md(&doc).unwrap();
//! assert!(md.contains("# Hello"));
//! ```
//!
//! Unknown ADF nodes round-trip losslessly via [`Node::Unknown`], so documents
//! using future Atlassian schema additions don't need a converter update.

pub mod adf;
pub mod adf_to_md;
pub mod md_to_adf;

#[cfg(feature = "python")]
mod py;

pub use adf::model::*;

/// Errors that can occur during conversion.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("ADF parse error: {0}")]
    AdfParse(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convert a Markdown string to an ADF [`Document`].
pub fn md_to_adf(markdown: &str) -> Result<Document, Error> {
    md_to_adf::convert(markdown)
}

/// Convert an ADF [`Document`] to a Markdown string.
pub fn adf_to_md(doc: &Document) -> Result<String, Error> {
    adf_to_md::convert(doc)
}

/// Convert a Markdown string to ADF JSON.
pub fn md_to_adf_json(markdown: &str) -> Result<String, Error> {
    let doc = md_to_adf(markdown)?;
    Ok(serde_json::to_string(&doc)?)
}

/// Convert ADF JSON to a Markdown string.
pub fn adf_json_to_md(json: &str) -> Result<String, Error> {
    let doc: Document = serde_json::from_str(json)?;
    adf_to_md(&doc)
}
