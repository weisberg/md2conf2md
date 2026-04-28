//! Unit tests for individual ADF node types.

use md2conf2md::adf::model::*;
use md2conf2md::{adf_to_md, md_to_adf};
use pretty_assertions::assert_eq;

#[test]
fn simple_paragraph() {
    let doc = md_to_adf("Hello world.\n").unwrap();
    assert_eq!(doc.content.len(), 1);
    match &doc.content[0] {
        Node::Paragraph { content } => {
            assert_eq!(content.len(), 1);
            match &content[0] {
                Node::Text { text, marks } => {
                    assert_eq!(text, "Hello world.");
                    assert!(marks.is_empty());
                }
                other => panic!("expected Text, got {other:?}"),
            }
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn heading_levels() {
    for level in 1..=6u8 {
        let md = format!("{} Title\n", "#".repeat(level as usize));
        let doc = md_to_adf(&md).unwrap();
        match &doc.content[0] {
            Node::Heading { attrs, content } => {
                assert_eq!(attrs.level, level);
                assert_eq!(content.len(), 1);
            }
            other => panic!("expected Heading, got {other:?}"),
        }
    }
}

#[test]
fn bold_text() {
    let doc = md_to_adf("**bold**\n").unwrap();
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "bold");
            assert!(marks.contains(&Mark::Strong));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn italic_text() {
    let doc = md_to_adf("*italic*\n").unwrap();
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "italic");
            assert!(marks.contains(&Mark::Em));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn strikethrough_text() {
    let doc = md_to_adf("~~struck~~\n").unwrap();
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "struck");
            assert!(marks.contains(&Mark::Strike));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn inline_code() {
    let doc = md_to_adf("`code`\n").unwrap();
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "code");
            assert!(marks.contains(&Mark::Code));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn link_with_title() {
    let doc = md_to_adf("[text](https://example.com \"A title\")\n").unwrap();
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        if let Node::Text { marks, .. } = &content[0] {
            let link_mark = marks.iter().find(|m| matches!(m, Mark::Link { .. }));
            assert!(link_mark.is_some());
            if let Some(Mark::Link { attrs }) = link_mark {
                assert_eq!(attrs.href, "https://example.com");
                assert_eq!(attrs.title.as_deref(), Some("A title"));
            }
        }
    }
}

#[test]
fn bullet_list_items() {
    let doc = md_to_adf("- a\n- b\n- c\n").unwrap();
    match &doc.content[0] {
        Node::BulletList { content } => {
            assert_eq!(content.len(), 3);
        }
        other => panic!("expected BulletList, got {other:?}"),
    }
}

#[test]
fn ordered_list_start() {
    let doc = md_to_adf("3. a\n4. b\n").unwrap();
    match &doc.content[0] {
        Node::OrderedList { attrs, content } => {
            assert_eq!(attrs.as_ref().unwrap().order, 3);
            assert_eq!(content.len(), 2);
        }
        other => panic!("expected OrderedList, got {other:?}"),
    }
}

#[test]
fn code_block_with_language() {
    let doc = md_to_adf("```python\nprint(\"hi\")\n```\n").unwrap();
    match &doc.content[0] {
        Node::CodeBlock { attrs, content } => {
            assert_eq!(attrs.as_ref().unwrap().language.as_deref(), Some("python"));
            assert!(!content.is_empty());
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

#[test]
fn thematic_break() {
    let doc = md_to_adf("---\n").unwrap();
    assert!(matches!(&doc.content[0], Node::Rule));
}

#[test]
fn blockquote_content() {
    let doc = md_to_adf("> Quoted text\n").unwrap();
    match &doc.content[0] {
        Node::Blockquote { content } => {
            assert!(!content.is_empty());
        }
        other => panic!("expected Blockquote, got {other:?}"),
    }
}

#[test]
fn task_list_states() {
    let doc = md_to_adf("- [x] done\n- [ ] todo\n").unwrap();
    match &doc.content[0] {
        Node::TaskList { content, .. } => {
            assert_eq!(content.len(), 2);
            if let Node::TaskItem { attrs, .. } = &content[0] {
                assert_eq!(attrs.state, TaskState::Done);
            }
            if let Node::TaskItem { attrs, .. } = &content[1] {
                assert_eq!(attrs.state, TaskState::Todo);
            }
        }
        other => panic!("expected TaskList, got {other:?}"),
    }
}

#[test]
fn footnote_reference_and_definition() {
    let doc = md_to_adf("Hello[^1].\n\n[^1]: The footnote.\n").unwrap();
    // Paragraph with text + superscript reference
    let para = &doc.content[0];
    if let Node::Paragraph { content } = para {
        let has_sup = content.iter().any(|n| {
            matches!(n, Node::Text { marks, text }
                if text == "[1]"
                && marks.iter().any(|m| matches!(m, Mark::SubSup { attrs } if attrs.sub_sup_type == SubSupType::Sup)))
        });
        assert!(
            has_sup,
            "expected a sup-marked footnote ref, got {content:?}"
        );
    } else {
        panic!("expected Paragraph");
    }
    // Definition becomes a trailing paragraph with "(1) " prefix
    let def = doc.content.last().unwrap();
    if let Node::Paragraph { content } = def {
        let first = content.first().unwrap();
        if let Node::Text { text, .. } = first {
            assert!(text.starts_with("(1) "), "got: {text:?}");
        } else {
            panic!("expected Text as first child of definition");
        }
    }
}

#[test]
fn adf_model_json_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "Hello".to_string(),
            marks: vec![Mark::Strong],
        }],
    }]);
    let json = serde_json::to_string(&doc).unwrap();
    let parsed: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(doc, parsed);
}

#[test]
fn adf_to_md_simple() {
    let doc = Document::new(vec![
        Node::Heading {
            attrs: HeadingAttrs { level: 1 },
            content: vec![Node::Text {
                text: "Title".to_string(),
                marks: vec![],
            }],
        },
        Node::Paragraph {
            content: vec![Node::Text {
                text: "Body text.".to_string(),
                marks: vec![],
            }],
        },
    ]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("# Title"));
    assert!(md.contains("Body text."));
}

#[test]
fn adf_to_md_marks() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![
            Node::Text {
                text: "bold".to_string(),
                marks: vec![Mark::Strong],
            },
            Node::Text {
                text: " and ".to_string(),
                marks: vec![],
            },
            Node::Text {
                text: "italic".to_string(),
                marks: vec![Mark::Em],
            },
        ],
    }]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("**bold**"));
    assert!(md.contains("*italic*"));
}

#[test]
fn adf_to_md_status() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Status {
            attrs: StatusAttrs {
                text: "Done".to_string(),
                color: StatusColor::Green,
                local_id: None,
                style: None,
            },
        }],
    }]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains(":status[Done]{color=green}"));
}

#[test]
fn adf_to_md_panel() {
    let doc = Document::new(vec![Node::Panel {
        attrs: PanelAttrs {
            panel_type: PanelType::Warning,
        },
        content: vec![Node::Paragraph {
            content: vec![Node::Text {
                text: "Watch out!".to_string(),
                marks: vec![],
            }],
        }],
    }]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("```adf:panel type=warning"));
    assert!(md.contains("Watch out!"));
}

#[test]
fn unknown_node_raw_roundtrip() {
    let raw_json = r#"{"type":"unknownFuture","attrs":{"foo":"bar"},"content":[]}"#;
    let node: Node = serde_json::from_str(raw_json).unwrap();
    assert!(matches!(node, Node::Unknown(_)));

    let doc = Document::new(vec![node]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("```adf:raw"));
}

#[test]
fn hard_break_in_table_cell() {
    // A HardBreak inside a table cell must not break the pipe row.
    let doc = Document::new(vec![Node::Table {
        attrs: None,
        content: vec![
            Node::TableRow {
                content: vec![Node::TableHeader {
                    attrs: None,
                    content: vec![Node::Paragraph {
                        content: vec![Node::Text {
                            text: "Header".to_string(),
                            marks: vec![],
                        }],
                    }],
                }],
            },
            Node::TableRow {
                content: vec![Node::TableCell {
                    attrs: None,
                    content: vec![Node::Paragraph {
                        content: vec![
                            Node::Text {
                                text: "top".to_string(),
                                marks: vec![],
                            },
                            Node::HardBreak,
                            Node::Text {
                                text: "bottom".to_string(),
                                marks: vec![],
                            },
                        ],
                    }],
                }],
            },
        ],
    }]);
    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("top<br>bottom"), "got: {md}");
    // Each row should be a single line
    for line in md.lines().filter(|l| l.starts_with('|')) {
        assert!(!line.contains("\\\n"), "row wrapped: {line}");
    }

    let reparsed = md_to_adf(&md).unwrap();
    if let Node::Table { content, .. } = &reparsed.content[0] {
        if let Node::TableRow { content: cells } = &content[1] {
            if let Node::TableCell { content, .. } = &cells[0] {
                if let Node::Paragraph { content } = &content[0] {
                    assert!(
                        content.iter().any(|n| matches!(n, Node::HardBreak)),
                        "intermediate markdown:\n{md}\ncontent: {content:?}"
                    );
                } else {
                    panic!("expected paragraph");
                }
            } else {
                panic!("expected table cell");
            }
        } else {
            panic!("expected table row");
        }
    } else {
        panic!("expected table");
    }
}

#[test]
fn html_br_parses_as_hard_break() {
    let doc = md_to_adf("top<br>bottom\n").unwrap();
    if let Node::Paragraph { content } = &doc.content[0] {
        assert!(
            content.iter().any(|n| matches!(n, Node::HardBreak)),
            "got {content:?}"
        );
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn multiparagraph_table_cell_does_not_concatenate_text() {
    let doc = Document::new(vec![Node::Table {
        attrs: None,
        content: vec![
            Node::TableRow {
                content: vec![Node::TableHeader {
                    attrs: None,
                    content: vec![Node::Paragraph {
                        content: vec![Node::Text {
                            text: "Header".to_string(),
                            marks: vec![],
                        }],
                    }],
                }],
            },
            Node::TableRow {
                content: vec![Node::TableCell {
                    attrs: None,
                    content: vec![
                        Node::Paragraph {
                            content: vec![Node::Text {
                                text: "First.".to_string(),
                                marks: vec![],
                            }],
                        },
                        Node::Paragraph {
                            content: vec![Node::Text {
                                text: "Second.".to_string(),
                                marks: vec![],
                            }],
                        },
                    ],
                }],
            },
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    assert!(md.contains("First.<br>Second."), "got:\n{md}");
}

#[test]
fn media_single_attrs_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::MediaSingle {
            attrs: Some(MediaSingleAttrs {
                layout: Some("center".to_string()),
                width: Some(320.0),
            }),
            content: vec![Node::Media {
                attrs: MediaAttrs {
                    media_type: MediaType::External,
                    id: "https://example.com/image.png".to_string(),
                    collection: String::new(),
                    width: None,
                    height: None,
                    alt: Some("alt".to_string()),
                },
            }],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        if let Node::MediaSingle { attrs, .. } = &content[0] {
            let attrs = attrs.as_ref().expect("media attrs");
            assert_eq!(attrs.layout.as_deref(), Some("center"));
            assert_eq!(attrs.width, Some(320.0));
        } else {
            panic!("expected mediaSingle, got {:?}", content[0]);
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn media_inline_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::MediaInline {
            attrs: MediaAttrs {
                media_type: MediaType::External,
                id: "https://example.com/icon.png".to_string(),
                collection: String::new(),
                width: None,
                height: None,
                alt: Some("icon".to_string()),
            },
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        assert!(
            matches!(&content[0], Node::MediaInline { attrs }
                if attrs.id == "https://example.com/icon.png" && attrs.alt.as_deref() == Some("icon")),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn special_chars_in_text_are_escaped() {
    // Text with markdown metacharacters must round-trip without producing
    // accidental emphasis, links, or code spans.
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "use *args and _kwargs_ plus `raw`".to_string(),
            marks: vec![],
        }],
    }]);
    let md = adf_to_md(&doc).unwrap();
    // Re-parse — we should end up with one text node with the same content
    let re = md_to_adf(&md).unwrap();
    if let Node::Paragraph { content } = &re.content[0] {
        assert_eq!(content.len(), 1, "got {content:?}");
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "use *args and _kwargs_ plus `raw`");
            assert!(marks.is_empty());
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn span_directive_underline_color() {
    let doc = md_to_adf(":span[red text]{underline=1 color=#ff0000}\n").unwrap();
    if let Node::Paragraph { content } = &doc.content[0] {
        let text = content.iter().find_map(|n| {
            if let Node::Text { text, marks } = n {
                Some((text.clone(), marks.clone()))
            } else {
                None
            }
        });
        let (t, marks) = text.expect("expected text node");
        assert_eq!(t, "red text");
        assert!(marks.iter().any(|m| matches!(m, Mark::Underline)));
        assert!(marks
            .iter()
            .any(|m| matches!(m, Mark::TextColor { attrs } if attrs.color == "#ff0000")));
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn span_directive_preserves_parent_marks() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "important".to_string(),
            marks: vec![Mark::Strong, Mark::Underline],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "important");
            assert!(marks.iter().any(|m| matches!(m, Mark::Strong)));
            assert!(marks.iter().any(|m| matches!(m, Mark::Underline)));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn code_mark_with_backticks_roundtrips() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "tick`inside".to_string(),
            marks: vec![Mark::Code],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "tick`inside");
            assert!(marks.iter().any(|m| matches!(m, Mark::Code)));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn link_text_with_closing_bracket_roundtrips() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "a]b".to_string(),
            marks: vec![Mark::Link {
                attrs: LinkAttrs {
                    href: "https://example.com".to_string(),
                    title: None,
                    collection: None,
                    id: None,
                    occurence_key: None,
                },
            }],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, "a]b");
            assert!(marks.iter().any(|m| matches!(m, Mark::Link { .. })));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn inline_directive_bodies_with_brackets_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![
            Node::Status {
                attrs: StatusAttrs {
                    text: r#"Done ] \ soon"#.to_string(),
                    color: StatusColor::Green,
                    local_id: None,
                    style: None,
                },
            },
            Node::Text {
                text: " ".to_string(),
                marks: vec![],
            },
            Node::Placeholder {
                attrs: PlaceholderAttrs {
                    text: r#"fill ] \ gap"#.to_string(),
                },
            },
            Node::Text {
                text: " ".to_string(),
                marks: vec![],
            },
            Node::InlineCard {
                attrs: CardAttrs {
                    url: r#"https://example.com/a]b\c"#.to_string(),
                },
            },
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        assert!(
            matches!(&content[0], Node::Status { attrs } if attrs.text == r#"Done ] \ soon"#),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
        assert!(
            matches!(&content[2], Node::Placeholder { attrs } if attrs.text == r#"fill ] \ gap"#),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
        assert!(
            matches!(&content[4], Node::InlineCard { attrs } if attrs.url == r#"https://example.com/a]b\c"#),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn mention_attrs_with_quotes_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Mention {
            attrs: MentionAttrs {
                id: r#"account]id"#.to_string(),
                text: Some(r#"Jane "JJ" \ Doe"#.to_string()),
                access_level: None,
                user_type: None,
            },
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        assert!(
            matches!(&content[0], Node::Mention { attrs }
                if attrs.id == r#"account]id"# && attrs.text.as_deref() == Some(r#"Jane "JJ" \ Doe"#)),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn emoji_roundtrips_with_directive_syntax() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Emoji {
            attrs: EmojiAttrs {
                short_name: ":custom]emoji:".to_string(),
                id: Some(r#"emoji-"id""#.to_string()),
                text: Some(r#"Party \ time"#.to_string()),
            },
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        assert!(
            matches!(&content[0], Node::Emoji { attrs }
                if attrs.short_name == ":custom]emoji:"
                    && attrs.id.as_deref() == Some(r#"emoji-"id""#)
                    && attrs.text.as_deref() == Some(r#"Party \ time"#)),
            "intermediate markdown:\n{md}\ncontent: {content:?}"
        );
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn span_with_code_and_escaped_body_roundtrips() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: r#"tick`]body"#.to_string(),
            marks: vec![Mark::Code, Mark::Underline],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Paragraph { content } = &reparsed.content[0] {
        if let Node::Text { text, marks } = &content[0] {
            assert_eq!(text, r#"tick`]body"#, "intermediate markdown:\n{md}");
            assert!(marks.iter().any(|m| matches!(m, Mark::Code)));
            assert!(marks.iter().any(|m| matches!(m, Mark::Underline)));
        } else {
            panic!("expected Text");
        }
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn placeholder_directive_parses() {
    let doc = md_to_adf(":placeholder[fill me in]\n").unwrap();
    if let Node::Paragraph { content } = &doc.content[0] {
        let has_placeholder = content
            .iter()
            .any(|n| matches!(n, Node::Placeholder { attrs } if attrs.text == "fill me in"));
        assert!(has_placeholder, "got {content:?}");
    } else {
        panic!("expected paragraph");
    }
}

#[test]
fn extension_body_preserves_multiple_paragraphs() {
    let doc = Document::new(vec![Node::Panel {
        attrs: PanelAttrs {
            panel_type: PanelType::Info,
        },
        content: vec![
            Node::Paragraph {
                content: vec![Node::Text {
                    text: "First.".to_string(),
                    marks: vec![],
                }],
            },
            Node::Paragraph {
                content: vec![Node::Text {
                    text: "Second.".to_string(),
                    marks: vec![],
                }],
            },
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Panel { content, .. } = &reparsed.content[0] {
        assert_eq!(content.len(), 2, "intermediate markdown:\n{md}");
        assert!(content.iter().all(|n| matches!(n, Node::Paragraph { .. })));
    } else {
        panic!("expected panel");
    }
}

#[test]
fn task_item_preserves_multiple_paragraphs() {
    let doc = Document::new(vec![Node::TaskList {
        attrs: Some(TaskListAttrs {
            local_id: String::new(),
        }),
        content: vec![Node::TaskItem {
            attrs: TaskItemAttrs {
                local_id: String::new(),
                state: TaskState::Todo,
            },
            content: vec![
                Node::Paragraph {
                    content: vec![Node::Text {
                        text: "First.".to_string(),
                        marks: vec![],
                    }],
                },
                Node::Paragraph {
                    content: vec![Node::Text {
                        text: "Second.".to_string(),
                        marks: vec![],
                    }],
                },
            ],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::TaskList { content, .. } = &reparsed.content[0] {
        if let Node::TaskItem { content, .. } = &content[0] {
            assert_eq!(content.len(), 2, "intermediate markdown:\n{md}");
            assert!(content.iter().all(|n| matches!(n, Node::Paragraph { .. })));
        } else {
            panic!("expected task item");
        }
    } else {
        panic!("expected task list");
    }
}

#[test]
fn mixed_task_and_plain_bullets_stay_bullet_list() {
    let doc = md_to_adf("- [ ] task-shaped item\n- plain item\n").unwrap();
    if let Node::BulletList { content } = &doc.content[0] {
        assert_eq!(content.len(), 2);
        assert!(content.iter().all(|n| matches!(n, Node::ListItem { .. })));
    } else {
        panic!("expected mixed list to stay a BulletList");
    }
}

#[test]
fn expand_title_with_quotes_roundtrips() {
    let doc = Document::new(vec![Node::Expand {
        attrs: Some(ExpandAttrs {
            title: Some(r#"Quote "this" \ title"#.to_string()),
        }),
        content: vec![Node::Paragraph {
            content: vec![Node::Text {
                text: "Hidden.".to_string(),
                marks: vec![],
            }],
        }],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    if let Node::Expand { attrs, .. } = &reparsed.content[0] {
        assert_eq!(
            attrs.as_ref().and_then(|a| a.title.as_deref()),
            Some(r#"Quote "this" \ title"#),
            "intermediate markdown:\n{md}"
        );
    } else {
        panic!("expected expand");
    }
}

#[test]
fn extension_fence_parses() {
    let md = "```adf:ext extensionType=\"com.atlassian.confluence.macro.core\" extensionKey=\"toc\"\n```\n";
    let doc = md_to_adf(md).unwrap();
    match &doc.content[0] {
        Node::Extension { attrs, .. } => {
            assert_eq!(attrs.extension_key, "toc");
            assert_eq!(attrs.extension_type, "com.atlassian.confluence.macro.core");
        }
        other => panic!("expected Extension, got {other:?}"),
    }
}

#[test]
fn expand_inside_panel_becomes_nested() {
    let md = "```adf:panel type=info\nOuter.\n\n```adf:expand title=\"inner\"\nHidden.\n```\n```\n";
    let doc = md_to_adf(md).unwrap();
    if let Node::Panel { content, .. } = &doc.content[0] {
        let has_nested = content
            .iter()
            .any(|n| matches!(n, Node::NestedExpand { .. }));
        assert!(has_nested, "expected a NestedExpand, got {content:?}");
    } else {
        panic!("expected panel");
    }
}

#[test]
fn table_structure() {
    let doc = md_to_adf("| A | B |\n| --- | --- |\n| 1 | 2 |\n").unwrap();
    match &doc.content[0] {
        Node::Table { content, .. } => {
            assert_eq!(content.len(), 2); // header row + 1 body row
            if let Node::TableRow { content: cells } = &content[0] {
                assert!(matches!(&cells[0], Node::TableHeader { .. }));
            }
        }
        other => panic!("expected Table, got {other:?}"),
    }
}
