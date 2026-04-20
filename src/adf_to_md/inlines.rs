//! Convert ADF inline nodes to Markdown text.

use crate::adf::model::*;

/// Write a single inline ADF node to the output string.
pub fn write_inline(node: &Node, out: &mut String) {
    match node {
        Node::Text { text, marks } => {
            write_marked_text(text, marks, out);
        }
        Node::HardBreak => {
            out.push_str("\\\n");
        }
        Node::Emoji { attrs } => {
            // Emit as :shortName: — strip the colons if already present
            let name = attrs.short_name.trim_matches(':');
            out.push(':');
            out.push_str(name);
            out.push(':');
        }
        Node::Mention { attrs } => {
            let display = attrs.text.as_deref().unwrap_or(&attrs.id);
            out.push_str(&format!("@[{}]{{text=\"{display}\"}}", attrs.id));
        }
        Node::Date { attrs } => {
            let date = timestamp_to_date(&attrs.timestamp);
            out.push_str(&format!(":date[{date}]"));
        }
        Node::Status { attrs } => {
            let color = match attrs.color {
                StatusColor::Neutral => "neutral",
                StatusColor::Purple => "purple",
                StatusColor::Blue => "blue",
                StatusColor::Red => "red",
                StatusColor::Yellow => "yellow",
                StatusColor::Green => "green",
            };
            out.push_str(&format!(":status[{}]{{color={color}}}", attrs.text));
        }
        Node::InlineCard { attrs } => {
            out.push_str(&format!(":card[{}]{{type=inline}}", attrs.url));
        }
        Node::MediaInline { attrs } => {
            let alt = attrs.alt.as_deref().unwrap_or("");
            out.push_str(&format!("![{alt}]({})", attrs.id));
            out.push_str("{inline=1}");
        }
        Node::Placeholder { attrs } => {
            out.push_str(&format!(":placeholder[{}]", attrs.text));
        }
        Node::MediaSingle { attrs, content } => {
            // Inline mediaSingle (shouldn't normally happen, but handle it)
            for child in content {
                if let Node::Media {
                    attrs: media_attrs, ..
                } = child
                {
                    let alt = media_attrs.alt.as_deref().unwrap_or("");
                    out.push_str(&format!("![{alt}]({})", media_attrs.id));
                    let mut dir_attrs = Vec::new();
                    if let Some(ref sa) = attrs {
                        if let Some(ref layout) = sa.layout {
                            dir_attrs.push(format!("layout={layout}"));
                        }
                        if let Some(width) = sa.width {
                            dir_attrs.push(format!("width={width}"));
                        }
                    }
                    if !dir_attrs.is_empty() {
                        out.push_str(&format!("{{{}}}", dir_attrs.join(" ")));
                    }
                }
            }
        }
        _ => {
            // For any unexpected node at inline level, try to extract text
            if let Some(text) = extract_text(node) {
                out.push_str(&text);
            }
        }
    }
}

/// Write text with its marks applied.
fn write_marked_text(text: &str, marks: &[Mark], out: &mut String) {
    // Separate marks into CommonMark marks and directive-only marks.
    // Links wrap the rendered label last so they compose with both Markdown
    // marks and :span[...] directive marks.
    let mut cm_prefix = String::new();
    let mut cm_suffix = String::new();
    let mut directive_attrs: Vec<String> = Vec::new();
    let mut link_attrs: Option<&LinkAttrs> = None;
    let mut is_code = false;

    for mark in marks {
        match mark {
            Mark::Strong => {
                cm_prefix.push_str("**");
                cm_suffix.insert_str(0, "**");
            }
            Mark::Em => {
                cm_prefix.push('*');
                cm_suffix.insert(0, '*');
            }
            Mark::Code => is_code = true,
            Mark::Strike => {
                cm_prefix.push_str("~~");
                cm_suffix.insert_str(0, "~~");
            }
            Mark::Link { attrs } => {
                link_attrs.get_or_insert(attrs);
            }
            Mark::Underline => {
                directive_attrs.push("underline=1".to_string());
            }
            Mark::TextColor { attrs } => {
                directive_attrs.push(format!("color={}", attrs.color));
            }
            Mark::BackgroundColor { attrs } => {
                directive_attrs.push(format!("bg={}", attrs.color));
            }
            Mark::SubSup { attrs } => match attrs.sub_sup_type {
                SubSupType::Sub => directive_attrs.push("sub=1".to_string()),
                SubSupType::Sup => directive_attrs.push("sup=1".to_string()),
            },
            Mark::Border { .. } => {
                directive_attrs.push("border=1".to_string());
            }
            Mark::Annotation { .. } | Mark::Unknown(_) => {}
        }
    }

    let mut rendered = String::new();
    rendered.push_str(&cm_prefix);

    if !directive_attrs.is_empty() {
        rendered.push_str(&format!(":span[{text}]{{{}}}", directive_attrs.join(" ")));
    } else if is_code {
        rendered.push_str(&format_code_span(text));
    } else {
        rendered.push_str(&escape_md(text));
    }

    rendered.push_str(&cm_suffix);

    if let Some(attrs) = link_attrs {
        out.push('[');
        out.push_str(&rendered);
        if let Some(ref title) = attrs.title {
            let esc = title.replace('\\', "\\\\").replace('"', "\\\"");
            out.push_str(&format!("]({} \"{esc}\")", attrs.href));
        } else {
            out.push_str(&format!("]({})", attrs.href));
        }
    } else {
        out.push_str(&rendered);
    }
}

fn format_code_span(text: &str) -> String {
    let fence = "`".repeat(max_backtick_run(text) + 1);
    if text.starts_with('`') || text.ends_with('`') {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

fn max_backtick_run(text: &str) -> usize {
    let mut max_run = 0;
    let mut current = 0;
    for ch in text.chars() {
        if ch == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }
    max_run
}

/// Backslash-escape characters that would otherwise trigger Markdown parsing.
/// Conservatively targets the metacharacters most likely to introduce
/// ambiguity in plain text: `*`, `_`, `` ` ``, `[`, `]`, and `\`.
fn escape_md(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '*' | '_' | '`' | '[' | ']' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Try to extract plain text from any node (fallback).
fn extract_text(node: &Node) -> Option<String> {
    match node {
        Node::Text { text, .. } => Some(text.clone()),
        Node::Paragraph { content } => {
            let mut s = String::new();
            for n in content {
                if let Some(t) = extract_text(n) {
                    s.push_str(&t);
                }
            }
            Some(s)
        }
        _ => None,
    }
}

/// Convert a millisecond timestamp string to ISO date (yyyy-mm-dd).
fn timestamp_to_date(timestamp: &str) -> String {
    if let Ok(ms) = timestamp.parse::<i64>() {
        let days = ms / (86400 * 1000);
        let (y, m, d) = civil_from_days(days);
        return format!("{y:04}-{m:02}-{d:02}");
    }
    // If it's already a date string, return as-is
    timestamp.to_string()
}

/// Convert days since 1970-01-01 to (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
