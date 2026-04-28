//! Convert ADF block nodes to Markdown text.

use super::inlines::{escape_directive_body, quote_attr_value, write_inline};
use crate::adf::model::*;

/// Write block-level ADF nodes to the output string.
pub fn write_blocks(nodes: &[Node], out: &mut String, indent: &str) {
    let mut first = true;
    for node in nodes {
        if !first && needs_blank_line_before(node) {
            // Emit a blank separator line with the current indent prefix
            // so that blockquote continuation lines remain inside the quote
            out.push_str(indent.trim_end());
            out.push('\n');
        }
        first = false;
        write_block(node, out, indent);
    }
}

fn needs_blank_line_before(node: &Node) -> bool {
    matches!(
        node,
        Node::Paragraph { .. }
            | Node::Heading { .. }
            | Node::BulletList { .. }
            | Node::OrderedList { .. }
            | Node::TaskList { .. }
            | Node::DecisionList { .. }
            | Node::Blockquote { .. }
            | Node::CodeBlock { .. }
            | Node::Table { .. }
            | Node::Panel { .. }
            | Node::Expand { .. }
            | Node::NestedExpand { .. }
            | Node::LayoutSection { .. }
            | Node::MediaSingle { .. }
            | Node::MediaGroup { .. }
            | Node::BlockCard { .. }
            | Node::EmbedCard { .. }
            | Node::Extension { .. }
            | Node::BodiedExtension { .. }
            | Node::Rule
    )
}

/// Write a single block-level ADF node.
pub fn write_block(node: &Node, out: &mut String, indent: &str) {
    match node {
        Node::Paragraph { content } => {
            out.push_str(indent);
            write_inlines(content, out);
            out.push('\n');
        }
        Node::Heading { attrs, content } => {
            out.push_str(indent);
            for _ in 0..attrs.level {
                out.push('#');
            }
            out.push(' ');
            write_inlines(content, out);
            out.push('\n');
        }
        Node::BulletList { content } => {
            for item in content {
                write_list_item(item, "- ", out, indent);
            }
        }
        Node::OrderedList { attrs, content } => {
            let start = attrs.as_ref().map(|a| a.order).unwrap_or(1);
            for (i, item) in content.iter().enumerate() {
                let prefix = format!("{}. ", start as usize + i);
                write_list_item(item, &prefix, out, indent);
            }
        }
        Node::TaskList { content, .. } => {
            for item in content {
                if let Node::TaskItem { attrs, content } = item {
                    let prefix = match attrs.state {
                        TaskState::Done => "- [x] ",
                        TaskState::Todo => "- [ ] ",
                    };
                    write_prefixed_item_content(content, prefix, 2, out, indent);
                }
            }
        }
        Node::DecisionList { content, .. } => {
            for item in content {
                if let Node::DecisionItem { attrs, content } = item {
                    let prefix = match attrs.state {
                        DecisionState::Decided => "- [!] ",
                    };
                    write_prefixed_item_content(content, prefix, 2, out, indent);
                }
            }
        }
        Node::Blockquote { content } => {
            let new_indent = format!("{indent}> ");
            write_blocks(content, out, &new_indent);
        }
        Node::CodeBlock { attrs, content } => {
            let lang = attrs
                .as_ref()
                .and_then(|a| a.language.as_deref())
                .unwrap_or("");

            // Check if this is an adf: extension fence
            if lang.starts_with("adf:") {
                out.push_str(indent);
                out.push_str("```");
                out.push_str(lang);
                out.push('\n');
                for node in content {
                    if let Node::Text { text, .. } = node {
                        out.push_str(indent);
                        out.push_str(text);
                        out.push('\n');
                    }
                }
                out.push_str(indent);
                out.push_str("```\n");
                return;
            }

            out.push_str(indent);
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            for node in content {
                if let Node::Text { text, .. } = node {
                    for line in text.lines() {
                        out.push_str(indent);
                        out.push_str(line);
                        out.push('\n');
                    }
                }
            }
            out.push_str(indent);
            out.push_str("```\n");
        }
        Node::Rule => {
            out.push_str(indent);
            out.push_str("---\n");
        }
        Node::Table { content, .. } => {
            write_table(content, out, indent);
        }
        Node::Panel { attrs, content } => {
            let type_name = match attrs.panel_type {
                PanelType::Info => "info",
                PanelType::Note => "note",
                PanelType::Warning => "warning",
                PanelType::Success => "success",
                PanelType::Error => "error",
                PanelType::Custom => "custom",
            };
            let mut body = String::new();
            write_blocks_as_body(content, &mut body, indent);
            write_fenced_block(out, indent, &format!("adf:panel type={type_name}"), &body);
        }
        Node::Expand { attrs, content } | Node::NestedExpand { attrs, content } => {
            let info = if let Some(ref a) = attrs {
                if let Some(ref title) = a.title {
                    format!("adf:expand title={}", quote_attr_value(title))
                } else {
                    "adf:expand".to_string()
                }
            } else {
                "adf:expand".to_string()
            };
            let mut body = String::new();
            write_blocks_as_body(content, &mut body, indent);
            write_fenced_block(out, indent, &info, &body);
        }
        Node::LayoutSection { content } => {
            let widths: Vec<String> = content
                .iter()
                .filter_map(|n| {
                    if let Node::LayoutColumn { attrs, .. } = n {
                        Some(format!("{}", attrs.width))
                    } else {
                        None
                    }
                })
                .collect();
            let mut body = String::new();
            let mut first_col = true;
            for col in content {
                if let Node::LayoutColumn { content, .. } = col {
                    if !first_col {
                        body.push_str(indent);
                        body.push_str("---col---\n");
                        body.push('\n');
                    }
                    first_col = false;
                    write_blocks_as_body(content, &mut body, indent);
                }
            }
            write_fenced_block(
                out,
                indent,
                &format!("adf:layout widths={}", widths.join(",")),
                &body,
            );
        }
        Node::MediaSingle { attrs, content } => {
            // Find the media child
            for child in content {
                if let Node::Media { attrs: media_attrs } = child {
                    out.push_str(indent);
                    let alt = media_attrs.alt.as_deref().unwrap_or("");
                    out.push_str(&format!("![{alt}]({})", media_attrs.id));
                    // Emit directive attrs if present
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
                    out.push('\n');
                }
            }
        }
        Node::MediaGroup { content } => {
            out.push_str(indent);
            for (i, child) in content.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                if let Node::Media { attrs: media_attrs } = child {
                    let alt = media_attrs.alt.as_deref().unwrap_or("");
                    out.push_str(&format!("![{alt}]({})", media_attrs.id));
                }
            }
            out.push('\n');
        }
        Node::BlockCard { attrs } => {
            out.push_str(indent);
            out.push_str(&format!(
                ":card[{}]{{type=block}}\n",
                escape_directive_body(&attrs.url)
            ));
        }
        Node::EmbedCard { attrs } => {
            out.push_str(indent);
            let mut dir_attrs = vec!["type=embed".to_string()];
            if let Some(ref layout) = attrs.layout {
                dir_attrs.push(format!("layout={}", quote_attr_value(layout)));
            }
            if let Some(width) = attrs.width {
                dir_attrs.push(format!("width={width}"));
            }
            out.push_str(&format!(
                ":card[{}]{{{}}}\n",
                escape_directive_body(&attrs.url),
                dir_attrs.join(" ")
            ));
        }
        Node::Extension { attrs, .. } | Node::BodiedExtension { attrs, .. } => {
            let mut attr_parts = vec![
                format!("extensionType={}", quote_attr_value(&attrs.extension_type)),
                format!("extensionKey={}", quote_attr_value(&attrs.extension_key)),
            ];
            if let Some(ref params) = attrs.parameters {
                attr_parts.push(format!(
                    "parameters={}",
                    percent_encode_attr(&serde_json::to_string(params).unwrap_or_default())
                ));
            }
            let mut body = String::new();
            if let Node::BodiedExtension { content, .. } = node {
                write_blocks_as_body(content, &mut body, indent);
            }
            write_fenced_block(
                out,
                indent,
                &format!("adf:ext {}", attr_parts.join(" ")),
                &body,
            );
        }
        Node::Unknown(value) => {
            let mut body = String::new();
            let json = serde_json::to_string_pretty(value).unwrap_or_default();
            for line in json.lines() {
                body.push_str(indent);
                body.push_str(line);
                body.push('\n');
            }
            write_fenced_block(out, indent, "adf:raw", &body);
        }
        // Inline nodes at block level — wrap in implicit paragraph
        Node::Text { .. }
        | Node::HardBreak
        | Node::Emoji { .. }
        | Node::Mention { .. }
        | Node::Date { .. }
        | Node::Status { .. }
        | Node::InlineCard { .. } => {
            out.push_str(indent);
            write_inline(node, out);
            out.push('\n');
        }
        // Nodes handled by their parents
        Node::ListItem { .. }
        | Node::TaskItem { .. }
        | Node::DecisionItem { .. }
        | Node::TableRow { .. }
        | Node::TableHeader { .. }
        | Node::TableCell { .. }
        | Node::LayoutColumn { .. }
        | Node::Media { .. }
        | Node::MediaInline { .. }
        | Node::Placeholder { .. } => {}
    }
}

/// Write inline content from a vec of nodes.
fn write_inlines(nodes: &[Node], out: &mut String) {
    for node in nodes {
        write_inline(node, out);
    }
}

/// Write a list item with the given prefix.
fn write_list_item(item: &Node, prefix: &str, out: &mut String, indent: &str) {
    if let Node::ListItem { content } = item {
        write_prefixed_item_content(content, prefix, prefix.len(), out, indent);
    }
}

/// Write list-like item content with a Markdown marker prefix.
fn write_prefixed_item_content(
    content: &[Node],
    prefix: &str,
    continuation_width: usize,
    out: &mut String,
    indent: &str,
) {
    let cont_indent = format!("{indent}{}", " ".repeat(continuation_width));
    out.push_str(indent);
    out.push_str(prefix);

    if content.is_empty() {
        out.push('\n');
        return;
    }

    let rest = match &content[0] {
        Node::Paragraph {
            content: paragraph_content,
        } => {
            write_inlines(paragraph_content, out);
            out.push('\n');
            &content[1..]
        }
        _ if is_inline_node(&content[0]) => {
            let inline_count = content.iter().take_while(|n| is_inline_node(n)).count();
            write_inlines(&content[..inline_count], out);
            out.push('\n');
            &content[inline_count..]
        }
        first => {
            write_block(first, out, "");
            &content[1..]
        }
    };

    for child in rest {
        // Blank separator so comrak keeps sibling blocks distinct.
        out.push_str(cont_indent.trim_end());
        out.push('\n');
        write_block(child, out, &cont_indent);
    }
}

fn is_inline_node(node: &Node) -> bool {
    matches!(
        node,
        Node::Text { .. }
            | Node::HardBreak
            | Node::Emoji { .. }
            | Node::Mention { .. }
            | Node::Date { .. }
            | Node::Status { .. }
            | Node::InlineCard { .. }
            | Node::MediaInline { .. }
            | Node::Placeholder { .. }
    )
}

/// Write a GFM pipe table.
fn write_table(rows: &[Node], out: &mut String, indent: &str) {
    // Collect cell text for all rows to compute column widths
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut is_header = Vec::new();

    for row in rows {
        if let Node::TableRow { content } = row {
            let mut cells = Vec::new();
            let mut row_is_header = false;
            for cell in content {
                match cell {
                    Node::TableHeader { content, .. } => {
                        row_is_header = true;
                        let mut s = String::new();
                        write_inlines(&get_cell_inlines(content), &mut s);
                        cells.push(flatten_cell(&s));
                    }
                    Node::TableCell { content, .. } => {
                        let mut s = String::new();
                        write_inlines(&get_cell_inlines(content), &mut s);
                        cells.push(flatten_cell(&s));
                    }
                    _ => {}
                }
            }
            is_header.push(row_is_header);
            all_rows.push(cells);
        }
    }

    if all_rows.is_empty() {
        return;
    }

    let num_cols = all_rows.iter().map(|r| r.len()).max().unwrap_or(0);

    // Write header row
    if let Some(header) = all_rows.first() {
        out.push_str(indent);
        out.push('|');
        for i in 0..num_cols {
            let cell = header.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!(" {cell} |"));
        }
        out.push('\n');

        // Separator
        out.push_str(indent);
        out.push('|');
        for _ in 0..num_cols {
            out.push_str(" --- |");
        }
        out.push('\n');
    }

    // Write body rows (skip first if it was the header)
    let start = if is_header.first().copied().unwrap_or(false) {
        1
    } else {
        0
    };
    for row in &all_rows[start..] {
        out.push_str(indent);
        out.push('|');
        for i in 0..num_cols {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!(" {cell} |"));
        }
        out.push('\n');
    }
}

/// Flatten a rendered cell string so it fits on one pipe-table row:
/// hard breaks (\\\n) become `<br>`, any remaining newlines become spaces,
/// and literal `|` is escaped.
fn flatten_cell(s: &str) -> String {
    s.replace("\\\n", "<br>")
        .replace('\n', " ")
        .replace('|', "\\|")
        .trim()
        .to_string()
}

/// Extract inline nodes from table cell content (which wraps in paragraphs).
fn get_cell_inlines(content: &[Node]) -> Vec<Node> {
    let mut inlines = Vec::new();
    let mut seen_paragraph = false;
    for node in content {
        if let Node::Paragraph { content } = node {
            if seen_paragraph {
                inlines.push(Node::HardBreak);
            }
            seen_paragraph = true;
            inlines.extend(content.iter().cloned());
        }
    }
    if inlines.is_empty() {
        // Fallback: return content directly
        inlines = content.to_vec();
    }
    inlines
}

/// Write inner content as a body (for panels, expands, etc.)
/// This emits the content without leading blank lines.
fn write_blocks_as_body(content: &[Node], out: &mut String, indent: &str) {
    write_blocks(content, out, indent);
}

fn write_fenced_block(out: &mut String, indent: &str, info: &str, body: &str) {
    let fence = fence_for_body(body);
    out.push_str(indent);
    out.push_str(&fence);
    out.push_str(info);
    out.push('\n');
    out.push_str(body);
    out.push_str(indent);
    out.push_str(&fence);
    out.push('\n');
}

fn fence_for_body(body: &str) -> String {
    "`".repeat(max_backtick_run(body).max(2) + 1)
}

fn max_backtick_run(text: &str) -> usize {
    let mut max_run = 0usize;
    let mut current = 0usize;
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

fn percent_encode_attr(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}
