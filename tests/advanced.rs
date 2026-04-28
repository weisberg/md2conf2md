//! Advanced regression tests that combine several conversion features.

use md2conf2md::adf::model::*;
use md2conf2md::{adf_to_md, md_to_adf};
use pretty_assertions::assert_eq;

fn text_paragraph(text: &str) -> Node {
    Node::Paragraph {
        content: vec![Node::Text {
            text: text.to_string(),
            marks: vec![],
        }],
    }
}

fn external_media(id: &str, alt: &str) -> Node {
    Node::Media {
        attrs: MediaAttrs {
            media_type: MediaType::External,
            id: id.to_string(),
            collection: String::new(),
            width: None,
            height: None,
            alt: Some(alt.to_string()),
        },
    }
}

#[test]
fn paragraph_text_that_looks_like_markdown_blocks_roundtrips() {
    let cases = [
        "# not a heading",
        "## also text",
        "- not a bullet",
        "+ not a bullet",
        "* not emphasis",
        "1. not ordered",
        "22) not ordered",
        "> not a quote",
        "---",
        "~~~",
        "~~not strike~~",
    ];

    for text in cases {
        let doc = Document::new(vec![text_paragraph(text)]);
        let md = adf_to_md(&doc).unwrap();
        let reparsed = md_to_adf(&md).unwrap();
        assert_eq!(
            reparsed, doc,
            "failed to preserve paragraph text {text:?}; intermediate markdown:\n{md}"
        );
    }
}

#[test]
fn hard_break_lines_that_look_like_markdown_blocks_roundtrip() {
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![
            Node::Text {
                text: "intro".to_string(),
                marks: vec![],
            },
            Node::HardBreak,
            Node::Text {
                text: "# still text".to_string(),
                marks: vec![],
            },
            Node::HardBreak,
            Node::Text {
                text: "- still text".to_string(),
                marks: vec![],
            },
            Node::HardBreak,
            Node::Text {
                text: "1. still text".to_string(),
                marks: vec![],
            },
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    assert_eq!(reparsed, doc, "intermediate markdown:\n{md}");
}

#[test]
fn media_group_roundtrips_from_adf_markdown_adf() {
    let doc = Document::new(vec![Node::MediaGroup {
        content: vec![
            external_media("https://example.com/one.png", "one"),
            external_media("https://example.com/two.png", "two"),
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    assert_eq!(reparsed, doc, "intermediate markdown:\n{md}");
}

#[test]
fn bodied_extension_with_json_parameters_roundtrips() {
    let params = serde_json::json!({
        "macroParams": {
            "title": "A \"quoted\" title",
            "items": ["one", "two three"],
            "nested": { "enabled": true }
        }
    });
    let doc = Document::new(vec![Node::BodiedExtension {
        attrs: ExtensionAttrs {
            extension_type: "com.atlassian.confluence.macro.core".to_string(),
            extension_key: "details".to_string(),
            parameters: Some(params),
            text: None,
            layout: None,
            local_id: None,
        },
        content: vec![
            text_paragraph("First body paragraph."),
            text_paragraph("Second body paragraph."),
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    assert_eq!(reparsed, doc, "intermediate markdown:\n{md}");
}

#[test]
fn code_with_directive_marks_roundtrips() {
    // Code mark combined with directive-only marks (underline, color, etc.)
    // must not silently drop the Code mark — the prior implementation took
    // the directive branch and forgot the backticks. Now Code rides along
    // as a `code=1` attribute on the :span[...] directive.
    let cases: Vec<Vec<Mark>> = vec![
        vec![Mark::Code, Mark::Underline],
        vec![
            Mark::Code,
            Mark::TextColor {
                attrs: TextColorAttrs {
                    color: "#ff0000".to_string(),
                },
            },
        ],
        vec![
            Mark::Code,
            Mark::SubSup {
                attrs: SubSupAttrs {
                    sub_sup_type: SubSupType::Sup,
                },
            },
        ],
    ];

    for marks in cases {
        let doc = Document::new(vec![Node::Paragraph {
            content: vec![Node::Text {
                text: "snippet".to_string(),
                marks: marks.clone(),
            }],
        }]);
        let md = adf_to_md(&doc).unwrap();
        let reparsed = md_to_adf(&md).unwrap();
        // ADF marks are an unordered set, so compare without caring about
        // the order they end up in after the round-trip.
        let Node::Paragraph {
            content: reparsed_inlines,
        } = &reparsed.content[0]
        else {
            panic!("expected paragraph");
        };
        let Node::Text {
            text: reparsed_text,
            marks: reparsed_marks,
        } = &reparsed_inlines[0]
        else {
            panic!("expected text");
        };
        assert_eq!(reparsed_text, "snippet", "intermediate markdown:\n{md}");
        for expected in &marks {
            assert!(
                reparsed_marks.contains(expected),
                "missing mark {expected:?} in round-trip {reparsed_marks:?}; markdown:\n{md}"
            );
        }
        assert_eq!(
            reparsed_marks.len(),
            marks.len(),
            "extra marks after round-trip {reparsed_marks:?}; markdown:\n{md}"
        );
    }
}

#[test]
fn span_body_with_brackets_roundtrips() {
    // The :span[...] body can contain literal `]`, which would otherwise
    // terminate the directive parse early. Both the emitter and parser
    // must agree on the `\]` escape.
    let doc = Document::new(vec![Node::Paragraph {
        content: vec![Node::Text {
            text: "see issue [#42] for context".to_string(),
            marks: vec![Mark::Underline],
        }],
    }]);
    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();
    assert_eq!(reparsed, doc, "intermediate markdown:\n{md}");
}

#[test]
fn nested_layout_panel_expand_roundtrips() {
    let doc = Document::new(vec![Node::LayoutSection {
        content: vec![
            Node::LayoutColumn {
                attrs: LayoutColumnAttrs { width: 33.33 },
                content: vec![Node::Panel {
                    attrs: PanelAttrs {
                        panel_type: PanelType::Warning,
                    },
                    content: vec![text_paragraph("Panel body.")],
                }],
            },
            Node::LayoutColumn {
                attrs: LayoutColumnAttrs { width: 66.67 },
                content: vec![Node::NestedExpand {
                    attrs: Some(ExpandAttrs {
                        title: Some("More detail".to_string()),
                    }),
                    content: vec![text_paragraph("Hidden body.")],
                }],
            },
        ],
    }]);

    let md = adf_to_md(&doc).unwrap();
    let reparsed = md_to_adf(&md).unwrap();

    assert_eq!(reparsed, doc, "intermediate markdown:\n{md}");
}
