//! Markdown → ADF conversion.

pub mod blocks;
pub mod extensions;
pub mod inlines;

use comrak::{parse_document, Arena, Options};

use crate::adf::model::Document;
use crate::Error;

/// Convert a Markdown string to an ADF [`Document`].
pub fn convert(markdown: &str) -> Result<Document, Error> {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.front_matter_delimiter = Some("---".to_string());

    let root = parse_document(&arena, markdown, &options);
    let content = blocks::convert_children(root);

    // Post-process to expand extension microsyntax
    let content = extensions::expand_extensions(content);

    Ok(Document::new(content))
}
