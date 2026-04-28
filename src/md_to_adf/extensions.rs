//! Extension microsyntax: parse `adf:*` fenced blocks and inline directives
//! from the comrak AST, replacing them with the corresponding ADF nodes.

use crate::adf::model::*;

/// Post-process a list of ADF nodes, expanding extension microsyntax.
///
/// This looks for:
/// - CodeBlock nodes whose language starts with "adf:" — these become the
///   corresponding ADF block node (panel, expand, layout, raw, etc.)
/// - Text nodes containing inline directives like `:status[text]{attrs}`
pub fn expand_extensions(nodes: Vec<Node>) -> Vec<Node> {
    nodes.into_iter().flat_map(expand_node).collect()
}

fn expand_node(node: Node) -> Vec<Node> {
    match node {
        Node::CodeBlock { attrs, content } => {
            if let Some(ref cb_attrs) = attrs {
                if let Some(ref lang) = cb_attrs.language {
                    if let Some(rest) = lang.strip_prefix("adf:") {
                        return expand_adf_fence(rest, &content);
                    }
                }
            }
            vec![Node::CodeBlock { attrs, content }]
        }
        // Recurse into container nodes
        Node::Paragraph { content } => {
            let content = expand_inline_directives(content);
            // If the paragraph now consists solely of a block-level directive
            // (BlockCard, EmbedCard), promote it out of the paragraph wrapper.
            if content.len() == 1
                && matches!(content[0], Node::BlockCard { .. } | Node::EmbedCard { .. })
            {
                return content;
            }
            if let Some(media_group) = try_as_media_group(&content) {
                return vec![media_group];
            }
            vec![Node::Paragraph { content }]
        }
        Node::Heading { attrs, content } => {
            let content = expand_inline_directives(content);
            vec![Node::Heading { attrs, content }]
        }
        Node::BulletList { content } => {
            // Recurse first, then inspect whether every item begins with the
            // decision marker `[!] ` — if so, rewrite as a DecisionList.
            let content = expand_extensions(content);
            if let Some(decision) = try_as_decision_list(&content) {
                return vec![decision];
            }
            vec![Node::BulletList { content }]
        }
        Node::OrderedList { attrs, content } => {
            vec![Node::OrderedList {
                attrs,
                content: expand_extensions(content),
            }]
        }
        Node::ListItem { content } => {
            vec![Node::ListItem {
                content: expand_extensions(content),
            }]
        }
        Node::Blockquote { content } => {
            vec![Node::Blockquote {
                content: expand_extensions(content),
            }]
        }
        Node::Table { attrs, content } => {
            vec![Node::Table {
                attrs,
                content: expand_extensions(content),
            }]
        }
        Node::TableRow { content } => {
            vec![Node::TableRow {
                content: expand_extensions(content),
            }]
        }
        Node::TableHeader { attrs, content } => {
            vec![Node::TableHeader {
                attrs,
                content: expand_extensions(content),
            }]
        }
        Node::TableCell { attrs, content } => {
            vec![Node::TableCell {
                attrs,
                content: expand_extensions(content),
            }]
        }
        Node::TaskList { attrs, content } => {
            vec![Node::TaskList {
                attrs,
                content: expand_extensions(content),
            }]
        }
        Node::TaskItem { attrs, content } => {
            vec![Node::TaskItem {
                attrs,
                content: expand_extensions(content),
            }]
        }
        other => vec![other],
    }
}

/// Recursively rewrite top-level `Expand` nodes into `NestedExpand`. Used
/// when constructing the body of a Panel/Expand/LayoutColumn, since an
/// Expand nested inside another container must be `nestedExpand` per ADF.
fn nestify_expands(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
        if let Node::Expand { attrs, content } = node {
            let attrs = attrs.take();
            let content = std::mem::take(content);
            *node = Node::NestedExpand { attrs, content };
        }
    }
}

/// If every list item in `items` begins with a `[!] ` marker, convert the
/// whole bullet list into an ADF `decisionList`.
fn try_as_decision_list(items: &[Node]) -> Option<Node> {
    if items.is_empty() {
        return None;
    }
    let mut decisions = Vec::with_capacity(items.len());
    for item in items {
        let Node::ListItem { content } = item else {
            return None;
        };
        // The first child must be a paragraph whose first text node starts with "[!] "
        let Some(Node::Paragraph { content: inlines }) = content.first() else {
            return None;
        };
        let Some(Node::Text { text, marks }) = inlines.first() else {
            return None;
        };
        let stripped = text.strip_prefix("[!] ")?;
        let mut new_inlines = Vec::with_capacity(inlines.len());
        new_inlines.push(Node::Text {
            text: stripped.to_string(),
            marks: marks.clone(),
        });
        new_inlines.extend(inlines.iter().skip(1).cloned());
        decisions.push(Node::DecisionItem {
            attrs: DecisionItemAttrs {
                local_id: String::new(),
                state: DecisionState::Decided,
            },
            content: new_inlines,
        });
    }
    Some(Node::DecisionList {
        attrs: Some(DecisionListAttrs {
            local_id: String::new(),
        }),
        content: decisions,
    })
}

fn try_as_media_group(content: &[Node]) -> Option<Node> {
    let mut media = Vec::new();
    for node in content {
        match node {
            Node::MediaSingle { attrs, content } if attrs.is_none() && content.len() == 1 => {
                let Node::Media { attrs } = &content[0] else {
                    return None;
                };
                media.push(Node::Media {
                    attrs: attrs.clone(),
                });
            }
            Node::Text { text, marks } if marks.is_empty() && text.trim().is_empty() => {}
            _ => return None,
        }
    }

    if media.len() >= 2 {
        Some(Node::MediaGroup { content: media })
    } else {
        None
    }
}

/// Expand an `adf:<type>` fenced code block into the corresponding ADF node.
fn expand_adf_fence(type_and_attrs: &str, content: &[Node]) -> Vec<Node> {
    let (node_type, attr_str) = match type_and_attrs.split_once(' ') {
        Some((t, a)) => (t, a.trim()),
        None => (type_and_attrs, ""),
    };

    match node_type {
        "panel" => {
            let panel_type =
                parse_attr_value(attr_str, "type").unwrap_or_else(|| "info".to_string());
            let pt = match panel_type.as_str() {
                "note" => PanelType::Note,
                "warning" => PanelType::Warning,
                "success" => PanelType::Success,
                "error" => PanelType::Error,
                "custom" => PanelType::Custom,
                _ => PanelType::Info,
            };
            let body_text = extract_text_content(content);
            let mut body = parse_body_as_blocks(&body_text);
            nestify_expands(&mut body);
            vec![Node::Panel {
                attrs: PanelAttrs { panel_type: pt },
                content: body,
            }]
        }
        "expand" => {
            let title = parse_attr_value(attr_str, "title");
            let body_text = extract_text_content(content);
            let mut body = parse_body_as_blocks(&body_text);
            nestify_expands(&mut body);
            vec![Node::Expand {
                attrs: Some(ExpandAttrs { title }),
                content: body,
            }]
        }
        "layout" => {
            let widths_str =
                parse_attr_value(attr_str, "widths").unwrap_or_else(|| "50,50".to_string());
            let widths: Vec<f64> = widths_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            let body_text = extract_text_content(content);
            let columns: Vec<&str> = body_text.split("---col---").collect();
            let mut col_nodes = Vec::new();
            for (i, col_text) in columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(50.0);
                let mut col_content = parse_body_as_blocks(col_text.trim());
                nestify_expands(&mut col_content);
                col_nodes.push(Node::LayoutColumn {
                    attrs: LayoutColumnAttrs { width },
                    content: col_content,
                });
            }
            vec![Node::LayoutSection { content: col_nodes }]
        }
        "ext" => {
            let extension_type = parse_attr_value(attr_str, "extensionType").unwrap_or_default();
            let extension_key = parse_attr_value(attr_str, "extensionKey").unwrap_or_default();
            let parameters = parse_attr_value(attr_str, "parameters")
                .and_then(|s| parse_json_attr_value(&s).ok());
            let attrs = ExtensionAttrs {
                extension_type,
                extension_key,
                parameters,
                text: None,
                layout: None,
                local_id: None,
            };
            let body_text = extract_text_content(content);
            if body_text.trim().is_empty() {
                vec![Node::Extension {
                    attrs,
                    content: vec![],
                }]
            } else {
                let body = parse_body_as_blocks(&body_text);
                vec![Node::BodiedExtension {
                    attrs,
                    content: body,
                }]
            }
        }
        "raw" => {
            // The body is raw JSON — parse it as an ADF node
            let json_text = extract_text_content(content);
            match serde_json::from_str::<Node>(&json_text) {
                Ok(node) => vec![node],
                Err(_) => {
                    // If we can't parse it, wrap in a code block
                    vec![Node::CodeBlock {
                        attrs: Some(CodeBlockAttrs {
                            language: Some("json".to_string()),
                        }),
                        content: vec![Node::Text {
                            text: json_text,
                            marks: vec![],
                        }],
                    }]
                }
            }
        }
        _ => {
            // Unknown adf: type — preserve as code block
            vec![Node::CodeBlock {
                attrs: Some(CodeBlockAttrs {
                    language: Some(format!("adf:{type_and_attrs}")),
                }),
                content: content.to_vec(),
            }]
        }
    }
}

/// Expand inline directives in a list of inline nodes.
///
/// Recognizes patterns like:
///   :status[Done]{color=green}
///   :date[2026-04-20]
///   @[accountId]{text="Display Name"}
///   :emoji[shortname]
fn expand_inline_directives(nodes: Vec<Node>) -> Vec<Node> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            Node::Text { text, marks } => {
                let expanded = parse_inline_directives(&text, &marks);
                result.extend(expanded);
            }
            other => result.push(other),
        }
    }
    attach_media_directives(result)
}

fn attach_media_directives(nodes: Vec<Node>) -> Vec<Node> {
    let mut result = Vec::new();
    let mut iter = nodes.into_iter().peekable();

    while let Some(node) = iter.next() {
        match node {
            Node::MediaSingle { attrs, content } => {
                if let Some(Node::Text { text, marks }) = iter.peek() {
                    if marks.is_empty() {
                        if let Some((media_node, consumed)) =
                            apply_media_directive(attrs.clone(), content.clone(), text)
                        {
                            result.push(media_node);
                            let Node::Text { text, marks } = iter.next().expect("peeked text")
                            else {
                                unreachable!("peeked node was text");
                            };
                            if consumed < text.len() {
                                result.push(Node::Text {
                                    text: text[consumed..].to_string(),
                                    marks,
                                });
                            }
                            continue;
                        }
                    }
                }
                result.push(Node::MediaSingle { attrs, content });
            }
            other => result.push(other),
        }
    }

    result
}

fn apply_media_directive(
    attrs: Option<MediaSingleAttrs>,
    content: Vec<Node>,
    text: &str,
) -> Option<(Node, usize)> {
    let (attr_str, consumed) = parse_leading_attr_block(text)?;
    let inline = parse_attr_value(attr_str, "inline").is_some();
    let layout = parse_attr_value(attr_str, "layout");
    let width = parse_attr_value(attr_str, "width").and_then(|s| s.parse().ok());

    if !inline && layout.is_none() && width.is_none() {
        return None;
    }

    if inline {
        if let Some(Node::Media { attrs }) = content.first() {
            return Some((
                Node::MediaInline {
                    attrs: attrs.clone(),
                },
                consumed,
            ));
        }
    }

    let mut media_attrs = attrs.unwrap_or(MediaSingleAttrs {
        layout: None,
        width: None,
    });
    if layout.is_some() {
        media_attrs.layout = layout;
    }
    if width.is_some() {
        media_attrs.width = width;
    }

    Some((
        Node::MediaSingle {
            attrs: Some(media_attrs),
            content,
        },
        consumed,
    ))
}

fn parse_leading_attr_block(text: &str) -> Option<(&str, usize)> {
    let rest = text.strip_prefix('{')?;
    let end = find_attr_block_end(rest)?;
    Some((&text[1..end], end + 1))
}

/// Parse inline directive syntax from a text string.
fn parse_inline_directives(text: &str, marks: &[Mark]) -> Vec<Node> {
    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Try to find an inline directive
        if let Some(pos) = find_directive_start(remaining) {
            // Emit text before the directive
            if pos > 0 {
                result.push(Node::Text {
                    text: remaining[..pos].to_string(),
                    marks: marks.to_vec(),
                });
            }
            remaining = &remaining[pos..];

            // Try to parse the directive
            if let Some((node, consumed)) = try_parse_directive(remaining, marks) {
                result.push(node);
                remaining = &remaining[consumed..];
            } else {
                // Not a valid directive — emit the character and continue
                result.push(Node::Text {
                    text: remaining[..1].to_string(),
                    marks: marks.to_vec(),
                });
                remaining = &remaining[1..];
            }
        } else {
            // No more directives — emit remaining text
            result.push(Node::Text {
                text: remaining.to_string(),
                marks: marks.to_vec(),
            });
            break;
        }
    }

    result
}

/// Find the start position of a potential inline directive.
fn find_directive_start(text: &str) -> Option<usize> {
    for (i, ch) in text.char_indices() {
        if ch == ':' || ch == '@' {
            return Some(i);
        }
    }
    None
}

/// Try to parse a directive starting at the beginning of the string.
/// Returns the parsed Node and the number of bytes consumed.
fn try_parse_directive(text: &str, parent_marks: &[Mark]) -> Option<(Node, usize)> {
    if let Some(rest) = text.strip_prefix(":status[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let color = parse_attr_value(attrs, "color").unwrap_or_else(|| "neutral".to_string());
            let sc = match color.as_str() {
                "purple" => StatusColor::Purple,
                "blue" => StatusColor::Blue,
                "red" => StatusColor::Red,
                "yellow" => StatusColor::Yellow,
                "green" => StatusColor::Green,
                _ => StatusColor::Neutral,
            };
            Node::Status {
                attrs: StatusAttrs {
                    text: body.to_string(),
                    color: sc,
                    local_id: None,
                    style: None,
                },
            }
        })
        .map(|(node, consumed)| (node, consumed + ":status[".len()))
    } else if let Some(rest) = text.strip_prefix(":date[") {
        parse_bracketed_directive(rest, |body, _attrs| {
            // Convert ISO date to timestamp (milliseconds since epoch)
            // For simplicity, store as the date string; real impl would convert
            let timestamp = date_to_timestamp(body);
            Node::Date {
                attrs: DateAttrs { timestamp },
            }
        })
        .map(|(node, consumed)| (node, consumed + ":date[".len()))
    } else if let Some(rest) = text.strip_prefix(":span[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let mut marks: Vec<Mark> = parent_marks.to_vec();
            if parse_attr_value(attrs, "underline").is_some() {
                marks.push(Mark::Underline);
            }
            if let Some(c) = parse_attr_value(attrs, "color") {
                marks.push(Mark::TextColor {
                    attrs: TextColorAttrs { color: c },
                });
            }
            if let Some(c) = parse_attr_value(attrs, "bg") {
                marks.push(Mark::BackgroundColor {
                    attrs: BackgroundColorAttrs { color: c },
                });
            }
            if parse_attr_value(attrs, "code").is_some() {
                marks.push(Mark::Code);
            }
            if parse_attr_value(attrs, "sub").is_some() {
                marks.push(Mark::SubSup {
                    attrs: SubSupAttrs {
                        sub_sup_type: SubSupType::Sub,
                    },
                });
            }
            if parse_attr_value(attrs, "sup").is_some() {
                marks.push(Mark::SubSup {
                    attrs: SubSupAttrs {
                        sub_sup_type: SubSupType::Sup,
                    },
                });
            }
            if parse_attr_value(attrs, "border").is_some() {
                marks.push(Mark::Border {
                    attrs: BorderAttrs {
                        size: None,
                        color: None,
                    },
                });
            }
            Node::Text {
                text: body.to_string(),
                marks,
            }
        })
        .map(|(node, consumed)| (node, consumed + ":span[".len()))
    } else if let Some(rest) = text.strip_prefix(":placeholder[") {
        parse_bracketed_directive(rest, |body, _attrs| Node::Placeholder {
            attrs: PlaceholderAttrs {
                text: body.to_string(),
            },
        })
        .map(|(node, consumed)| (node, consumed + ":placeholder[".len()))
    } else if let Some(rest) = text.strip_prefix(":emoji[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let id = parse_attr_value(attrs, "id");
            let emoji_text = parse_attr_value(attrs, "text");
            Node::Emoji {
                attrs: EmojiAttrs {
                    short_name: format!(":{body}:"),
                    id,
                    text: emoji_text,
                },
            }
        })
        .map(|(node, consumed)| (node, consumed + ":emoji[".len()))
    } else if let Some(rest) = text.strip_prefix(":card[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let card_type = parse_attr_value(attrs, "type").unwrap_or_else(|| "inline".to_string());
            match card_type.as_str() {
                "block" => Node::BlockCard {
                    attrs: CardAttrs {
                        url: body.to_string(),
                    },
                },
                "embed" => {
                    let layout = parse_attr_value(attrs, "layout");
                    let width = parse_attr_value(attrs, "width").and_then(|s| s.parse().ok());
                    Node::EmbedCard {
                        attrs: EmbedCardAttrs {
                            url: body.to_string(),
                            layout,
                            width,
                        },
                    }
                }
                _ => Node::InlineCard {
                    attrs: CardAttrs {
                        url: body.to_string(),
                    },
                },
            }
        })
        .map(|(node, consumed)| (node, consumed + ":card[".len()))
    } else if let Some(rest) = text.strip_prefix("@[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let display = parse_attr_value(attrs, "text");
            Node::Mention {
                attrs: MentionAttrs {
                    id: body.to_string(),
                    text: display,
                    access_level: None,
                    user_type: None,
                },
            }
        })
        .map(|(node, consumed)| (node, consumed + "@[".len()))
    } else {
        None
    }
}

/// Parse `body]{key=value}` returning the Node from the callback and bytes consumed.
fn parse_bracketed_directive<F>(text: &str, make_node: F) -> Option<(Node, usize)>
where
    F: FnOnce(&str, &str) -> Node,
{
    let bracket_end = find_unescaped_char(text, ']')?;
    let body = unescape_directive_body(&text[..bracket_end]);
    let after_bracket = &text[bracket_end + 1..];

    let (attrs_str, total_consumed) = if let Some(stripped) = after_bracket.strip_prefix('{') {
        if let Some(brace_end) = find_attr_block_end(stripped) {
            let attrs = &after_bracket[1..brace_end];
            (attrs, bracket_end + 1 + brace_end + 1)
        } else {
            ("", bracket_end + 1)
        }
    } else {
        ("", bracket_end + 1)
    };

    Some((make_node(&body, attrs_str), total_consumed))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Extract a named attribute value from a `key=value key2="quoted value"` string.
fn parse_attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut pos = 0;
    while pos < attrs.len() {
        pos = skip_ascii_whitespace(attrs, pos);
        if pos >= attrs.len() {
            break;
        }

        let key_start = pos;
        while pos < attrs.len() {
            let ch = attrs[pos..].chars().next()?;
            if ch == '=' || ch.is_ascii_whitespace() {
                break;
            }
            pos += ch.len_utf8();
        }
        let attr_key = &attrs[key_start..pos];
        pos = skip_ascii_whitespace(attrs, pos);
        if !attrs[pos..].starts_with('=') {
            while pos < attrs.len() {
                let ch = attrs[pos..].chars().next()?;
                pos += ch.len_utf8();
                if ch.is_ascii_whitespace() {
                    break;
                }
            }
            continue;
        }
        pos += 1;
        pos = skip_ascii_whitespace(attrs, pos);

        let (value, next_pos) = parse_attr_token(attrs, pos)?;
        if attr_key == key {
            return Some(value);
        }
        pos = next_pos;
    }
    None
}

fn parse_attr_token(attrs: &str, start: usize) -> Option<(String, usize)> {
    if attrs[start..].starts_with('"') {
        let value_start = start + 1;
        let quote_end = find_unescaped_char(&attrs[value_start..], '"')?;
        let raw = &attrs[value_start..value_start + quote_end];
        return Some((unescape_quoted_attr(raw), value_start + quote_end + 1));
    }

    let mut escaped = false;
    let mut in_quote = false;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (offset, ch) in attrs[start..].char_indices() {
        let pos = start + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote {
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                _ if ch.is_ascii_whitespace() && brace_depth == 0 && bracket_depth == 0 => {
                    return Some((attrs[start..pos].to_string(), pos));
                }
                _ => {}
            }
        }
    }

    Some((attrs[start..].to_string(), attrs.len()))
}

fn skip_ascii_whitespace(text: &str, mut pos: usize) -> usize {
    while pos < text.len() {
        let ch = text[pos..].chars().next().expect("valid char boundary");
        if !ch.is_ascii_whitespace() {
            break;
        }
        pos += ch.len_utf8();
    }
    pos
}

fn find_unescaped_char(text: &str, target: char) -> Option<usize> {
    let mut escaped = false;
    for (i, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == target {
            return Some(i);
        }
    }
    None
}

fn find_attr_block_end(text: &str) -> Option<usize> {
    let mut escaped = false;
    let mut in_quote = false;
    for (i, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if ch == '}' && !in_quote {
            return Some(i + 1);
        }
    }
    None
}

fn unescape_directive_body(text: &str) -> String {
    unescape_chars(text, &[']', '\\'])
}

fn unescape_quoted_attr(text: &str) -> String {
    unescape_chars(text, &['"', '\\'])
}

fn unescape_chars(text: &str, escapable: &[char]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                if escapable.contains(&next) {
                    out.push(next);
                } else {
                    out.push('\\');
                    out.push(next);
                }
            } else {
                out.push('\\');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_json_attr_value(value: &str) -> Result<serde_json::Value, serde_json::Error> {
    match serde_json::from_str(value) {
        Ok(json) => Ok(json),
        Err(err) => {
            if let Some(decoded) = percent_decode_attr(value) {
                serde_json::from_str(&decoded)
            } else {
                Err(err)
            }
        }
    }
}

fn percent_decode_attr(value: &str) -> Option<String> {
    if !value.as_bytes().contains(&b'%') {
        return None;
    }

    let mut bytes = Vec::with_capacity(value.len());
    let mut i = 0usize;
    let src = value.as_bytes();
    while i < src.len() {
        if src[i] == b'%' {
            if i + 2 >= src.len() {
                return None;
            }
            let hi = hex_value(src[i + 1])?;
            let lo = hex_value(src[i + 2])?;
            bytes.push((hi << 4) | lo);
            i += 3;
        } else {
            bytes.push(src[i]);
            i += 1;
        }
    }

    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Extract concatenated text from content nodes.
fn extract_text_content(nodes: &[Node]) -> String {
    let mut s = String::new();
    for node in nodes {
        if let Node::Text { text, .. } = node {
            s.push_str(text);
        }
    }
    s
}

/// Parse a markdown body string into ADF block nodes.
/// This is a simplified parser for content inside extension fences.
fn parse_body_as_blocks(text: &str) -> Vec<Node> {
    if text.trim().is_empty() {
        return vec![];
    }
    // Re-enter the full md_to_adf pipeline for the body
    match crate::md_to_adf::convert(text) {
        Ok(doc) => doc.content,
        Err(_) => {
            vec![Node::Paragraph {
                content: vec![Node::Text {
                    text: text.to_string(),
                    marks: vec![],
                }],
            }]
        }
    }
}

/// Convert an ISO date string (yyyy-mm-dd) to a millisecond timestamp string.
fn date_to_timestamp(date: &str) -> String {
    // Parse yyyy-mm-dd and compute ms since epoch (UTC midnight)
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i64>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            // Simple days-since-epoch calculation
            let days = days_from_civil(y, m, d);
            let ms = days * 86400 * 1000;
            return ms.to_string();
        }
    }
    date.to_string()
}

/// Days from 1970-01-01 (civil date to days since epoch).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy;
    era * 146097 + doe - 719468
}
