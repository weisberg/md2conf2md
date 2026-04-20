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

/// Expand an `adf:<type>` fenced code block into the corresponding ADF node.
fn expand_adf_fence(type_and_attrs: &str, content: &[Node]) -> Vec<Node> {
    let (node_type, attr_str) = match type_and_attrs.split_once(' ') {
        Some((t, a)) => (t, a.trim()),
        None => (type_and_attrs, ""),
    };

    match node_type {
        "panel" => {
            let panel_type = parse_attr_value(attr_str, "type").unwrap_or("info");
            let pt = match panel_type {
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
            let title = parse_attr_value(attr_str, "title").map(|s| unquote(s).to_string());
            let body_text = extract_text_content(content);
            let mut body = parse_body_as_blocks(&body_text);
            nestify_expands(&mut body);
            vec![Node::Expand {
                attrs: Some(ExpandAttrs { title }),
                content: body,
            }]
        }
        "layout" => {
            let widths_str = parse_attr_value(attr_str, "widths").unwrap_or("50,50");
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
            let extension_type = parse_attr_value(attr_str, "extensionType")
                .map(|s| unquote(s).to_string())
                .unwrap_or_default();
            let extension_key = parse_attr_value(attr_str, "extensionKey")
                .map(|s| unquote(s).to_string())
                .unwrap_or_default();
            let parameters = parse_attr_value(attr_str, "parameters")
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
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
    result
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
            if let Some((node, consumed)) = try_parse_directive(remaining) {
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
fn try_parse_directive(text: &str) -> Option<(Node, usize)> {
    if let Some(rest) = text.strip_prefix(":status[") {
        parse_bracketed_directive(rest, |body, attrs| {
            let color = parse_attr_value(attrs, "color").unwrap_or("neutral");
            let sc = match color {
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
            let mut marks: Vec<Mark> = Vec::new();
            if parse_attr_value(attrs, "underline").is_some() {
                marks.push(Mark::Underline);
            }
            if let Some(c) = parse_attr_value(attrs, "color") {
                marks.push(Mark::TextColor {
                    attrs: TextColorAttrs {
                        color: c.to_string(),
                    },
                });
            }
            if let Some(c) = parse_attr_value(attrs, "bg") {
                marks.push(Mark::BackgroundColor {
                    attrs: BackgroundColorAttrs {
                        color: c.to_string(),
                    },
                });
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
            let id = parse_attr_value(attrs, "id").map(|s| s.to_string());
            let emoji_text = parse_attr_value(attrs, "text").map(|s| s.to_string());
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
            let card_type = parse_attr_value(attrs, "type").unwrap_or("inline");
            match card_type {
                "block" => Node::BlockCard {
                    attrs: CardAttrs {
                        url: body.to_string(),
                    },
                },
                "embed" => {
                    let layout = parse_attr_value(attrs, "layout").map(|s| s.to_string());
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
            let display = parse_attr_value(attrs, "text").map(|s| unquote(s).to_string());
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
    let bracket_end = text.find(']')?;
    let body = &text[..bracket_end];
    let after_bracket = &text[bracket_end + 1..];

    let (attrs_str, total_consumed) = if after_bracket.starts_with('{') {
        if let Some(brace_end) = after_bracket.find('}') {
            let attrs = &after_bracket[1..brace_end];
            (attrs, bracket_end + 1 + brace_end + 1)
        } else {
            ("", bracket_end + 1)
        }
    } else {
        ("", bracket_end + 1)
    };

    Some((make_node(body, attrs_str), total_consumed))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Extract a named attribute value from a `key=value key2="quoted value"` string.
fn parse_attr_value<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let search = format!("{key}=");
    let start = attrs.find(&search)?;
    let val_start = start + search.len();
    let rest = &attrs[val_start..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(&stripped[..end])
    } else {
        let end = rest.find(' ').unwrap_or(rest.len());
        Some(&rest[..end])
    }
}

/// Remove surrounding quotes from a string.
fn unquote(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
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
