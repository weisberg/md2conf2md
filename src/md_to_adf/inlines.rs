//! Convert comrak inline AST nodes to ADF inline nodes.

use comrak::arena_tree::Node as ArenaNode;
use comrak::nodes::{Ast, NodeValue};
use std::cell::RefCell;

use crate::adf::model::*;

/// Convert all children of a node as inline content.
pub fn convert_inline_children<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>) -> Vec<Node> {
    let mut result = Vec::new();
    for child in node.children() {
        convert_inline(child, &[], &mut result);
    }
    merge_adjacent_text(result)
}

fn merge_adjacent_text(nodes: Vec<Node>) -> Vec<Node> {
    let mut merged: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::Text { text, marks } => {
                if let Some(Node::Text {
                    text: previous_text,
                    marks: previous_marks,
                }) = merged.last_mut()
                {
                    if *previous_marks == marks {
                        previous_text.push_str(&text);
                        continue;
                    }
                }
                merged.push(Node::Text { text, marks });
            }
            other => merged.push(other),
        }
    }
    merged
}

/// Collect the concatenated plain text of all descendant Text/Code nodes.
fn collect_text<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>, out: &mut String) {
    for child in node.children() {
        let ast = child.data.borrow();
        match &ast.value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
            _ => collect_text(child, out),
        }
    }
}

/// Convert a single inline node, accumulating marks from parent contexts.
fn convert_inline<'a>(
    node: &'a ArenaNode<'a, RefCell<Ast>>,
    parent_marks: &[Mark],
    out: &mut Vec<Node>,
) {
    let ast = node.data.borrow();
    match &ast.value {
        NodeValue::Text(text) => {
            out.push(Node::Text {
                text: text.clone(),
                marks: parent_marks.to_vec(),
            });
        }
        NodeValue::Code(code) => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::Code);
            out.push(Node::Text {
                text: code.literal.clone(),
                marks,
            });
        }
        NodeValue::SoftBreak => {
            // ADF treats soft breaks as a space
            out.push(Node::Text {
                text: " ".to_string(),
                marks: parent_marks.to_vec(),
            });
        }
        NodeValue::LineBreak => {
            out.push(Node::HardBreak);
        }
        NodeValue::HtmlInline(html) if is_html_break(html) => {
            out.push(Node::HardBreak);
        }
        NodeValue::HtmlInline(html) => {
            out.push(Node::Text {
                text: html.clone(),
                marks: parent_marks.to_vec(),
            });
        }
        NodeValue::Emph => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::Em);
            for child in node.children() {
                convert_inline(child, &marks, out);
            }
        }
        NodeValue::Strong => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::Strong);
            for child in node.children() {
                convert_inline(child, &marks, out);
            }
        }
        NodeValue::Strikethrough => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::Strike);
            for child in node.children() {
                convert_inline(child, &marks, out);
            }
        }
        NodeValue::Link(link) => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::Link {
                attrs: LinkAttrs {
                    href: link.url.clone(),
                    title: if link.title.is_empty() {
                        None
                    } else {
                        Some(link.title.clone())
                    },
                    collection: None,
                    id: None,
                    occurence_key: None,
                },
            });
            for child in node.children() {
                convert_inline(child, &marks, out);
            }
        }
        NodeValue::Image(link) => {
            // Alt text lives in the Image's descendant Text nodes (`![alt](url)`);
            // link.title is the optional title string (`![alt](url "title")`).
            let mut alt_text = String::new();
            collect_text(node, &mut alt_text);
            let media = Node::Media {
                attrs: MediaAttrs {
                    media_type: MediaType::External,
                    id: link.url.clone(),
                    collection: String::new(),
                    width: None,
                    height: None,
                    alt: if alt_text.is_empty() {
                        None
                    } else {
                        Some(alt_text)
                    },
                },
            };
            out.push(Node::MediaSingle {
                attrs: None,
                content: vec![media],
            });
        }
        // Paragraph inside inline context — just recurse into children
        NodeValue::Paragraph => {
            for child in node.children() {
                convert_inline(child, parent_marks, out);
            }
        }
        NodeValue::FootnoteReference(fr) => {
            let mut marks = parent_marks.to_vec();
            marks.push(Mark::SubSup {
                attrs: SubSupAttrs {
                    sub_sup_type: SubSupType::Sup,
                },
            });
            out.push(Node::Text {
                text: format!("[{}]", fr.name),
                marks,
            });
        }
        _ => {
            // Fallback: recurse children preserving marks
            for child in node.children() {
                convert_inline(child, parent_marks, out);
            }
        }
    }
}

fn is_html_break(html: &str) -> bool {
    matches!(
        html.trim().to_ascii_lowercase().as_str(),
        "<br>" | "<br/>" | "<br />"
    )
}
