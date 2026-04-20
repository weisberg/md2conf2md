//! Convert comrak block-level AST nodes to ADF nodes.

use comrak::arena_tree::Node as ArenaNode;
use comrak::nodes::{Ast, ListType, NodeValue};
use std::cell::RefCell;

use super::inlines::convert_inline_children;
use crate::adf::model::*;

/// Convert a single comrak block node and its children to ADF node(s).
pub fn convert_block<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>) -> Vec<Node> {
    let ast = node.data.borrow();
    match &ast.value {
        NodeValue::Document => {
            // Document's children are processed by the caller
            convert_children(node)
        }
        NodeValue::Paragraph => {
            let content = convert_inline_children(node);
            vec![Node::Paragraph { content }]
        }
        NodeValue::Heading(heading) => {
            let content = convert_inline_children(node);
            vec![Node::Heading {
                attrs: HeadingAttrs {
                    level: heading.level,
                },
                content,
            }]
        }
        NodeValue::BlockQuote => {
            let content = convert_children(node);
            vec![Node::Blockquote { content }]
        }
        NodeValue::List(list) => match list.list_type {
            ListType::Bullet => {
                let mut has_task_items = false;
                let mut all_task_items = true;
                for child in node.children() {
                    let child_ast = child.data.borrow();
                    let is_task = matches!(child_ast.value, NodeValue::TaskItem(_));
                    has_task_items |= is_task;
                    all_task_items &= is_task;
                }

                if has_task_items && all_task_items {
                    vec![Node::TaskList {
                        attrs: Some(TaskListAttrs {
                            local_id: String::new(),
                        }),
                        content: convert_children(node),
                    }]
                } else {
                    let content = if has_task_items {
                        convert_mixed_task_list_children(node)
                    } else {
                        convert_children(node)
                    };
                    vec![Node::BulletList { content }]
                }
            }
            ListType::Ordered => {
                let items = convert_children(node);
                let attrs = if list.start != 1 {
                    Some(OrderedListAttrs {
                        order: list.start as u32,
                    })
                } else {
                    None
                };
                vec![Node::OrderedList {
                    attrs,
                    content: items,
                }]
            }
        },
        NodeValue::Item(_) => {
            let content = convert_children(node);
            vec![Node::ListItem { content }]
        }
        NodeValue::TaskItem(checked) => {
            let state = if checked.is_some() {
                TaskState::Done
            } else {
                TaskState::Todo
            };
            let content = convert_children(node);
            vec![Node::TaskItem {
                attrs: TaskItemAttrs {
                    local_id: String::new(),
                    state,
                },
                content,
            }]
        }
        NodeValue::CodeBlock(code_block) => {
            let language = if code_block.info.is_empty() {
                None
            } else if code_block.info.starts_with("adf:") {
                // Preserve the full info string for extension processing
                Some(code_block.info.clone())
            } else {
                // Standard code block: take just the language token
                let lang = code_block.info.split_whitespace().next().unwrap_or("");
                if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_string())
                }
            };
            let text = code_block.literal.clone();
            // ADF codeBlock wraps text content in a text node
            let content = if text.is_empty() {
                vec![]
            } else {
                // Strip trailing newline that comrak adds
                let text = text.strip_suffix('\n').unwrap_or(&text).to_string();
                vec![Node::Text {
                    text,
                    marks: vec![],
                }]
            };
            vec![Node::CodeBlock {
                attrs: Some(CodeBlockAttrs { language }),
                content,
            }]
        }
        NodeValue::ThematicBreak => {
            vec![Node::Rule]
        }
        NodeValue::Table(_) => {
            let content = convert_children(node);
            vec![Node::Table {
                attrs: None,
                content,
            }]
        }
        NodeValue::TableRow(header) => {
            let cells: Vec<Node> = node
                .children()
                .flat_map(|child| {
                    let child_ast = child.data.borrow();
                    match &child_ast.value {
                        NodeValue::TableCell => {
                            let cell_content = convert_inline_children(child);
                            // Wrap inline content in a paragraph (ADF requires block content in cells)
                            let para = Node::Paragraph {
                                content: cell_content,
                            };
                            if *header {
                                vec![Node::TableHeader {
                                    attrs: None,
                                    content: vec![para],
                                }]
                            } else {
                                vec![Node::TableCell {
                                    attrs: None,
                                    content: vec![para],
                                }]
                            }
                        }
                        _ => convert_block(child),
                    }
                })
                .collect();
            vec![Node::TableRow { content: cells }]
        }
        NodeValue::TableCell => {
            // Handled by TableRow above
            vec![]
        }
        // Footnote definition: emit as a paragraph prefixed with [name]:
        // so it renders readably. ADF has no native footnote node.
        NodeValue::FootnoteDefinition(fd) => {
            // ADF has no footnote node. Flatten each definition to a
            // paragraph whose first token is the footnote label.
            let mut content = vec![Node::Text {
                text: format!("({}) ", fd.name),
                marks: vec![],
            }];
            for child in node.children() {
                let child_ast = child.data.borrow();
                if let NodeValue::Paragraph = &child_ast.value {
                    content.extend(convert_inline_children(child));
                }
            }
            vec![Node::Paragraph { content }]
        }
        // Nodes we pass through by converting children
        NodeValue::FrontMatter(_) => vec![],
        _ => {
            // For any other block-level node, try to convert children
            convert_children(node)
        }
    }
}

/// Recursively convert all children of a node.
pub fn convert_children<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>) -> Vec<Node> {
    node.children()
        .flat_map(|child| convert_block(child))
        .collect()
}

fn convert_mixed_task_list_children<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>) -> Vec<Node> {
    node.children()
        .flat_map(|child| {
            let task_state = {
                let child_ast = child.data.borrow();
                if let NodeValue::TaskItem(checked) = &child_ast.value {
                    Some(checked.is_some())
                } else {
                    None
                }
            };

            if let Some(done) = task_state {
                vec![convert_task_item_as_list_item(child, done)]
            } else {
                convert_block(child)
            }
        })
        .collect()
}

fn convert_task_item_as_list_item<'a>(node: &'a ArenaNode<'a, RefCell<Ast>>, done: bool) -> Node {
    let mut content = convert_children(node);
    let marker = if done { "[x] " } else { "[ ] " };
    prepend_text_marker(&mut content, marker);
    Node::ListItem { content }
}

fn prepend_text_marker(content: &mut Vec<Node>, marker: &str) {
    let text = Node::Text {
        text: marker.to_string(),
        marks: vec![],
    };

    if let Some(Node::Paragraph { content }) = content.first_mut() {
        content.insert(0, text);
    } else {
        content.insert(
            0,
            Node::Paragraph {
                content: vec![text],
            },
        );
    }
}
