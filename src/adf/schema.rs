//! ADF schema constants and helpers.

/// All known ADF block node type names.
pub const BLOCK_NODES: &[&str] = &[
    "paragraph",
    "heading",
    "bulletList",
    "orderedList",
    "listItem",
    "taskList",
    "taskItem",
    "decisionList",
    "decisionItem",
    "blockquote",
    "codeBlock",
    "rule",
    "table",
    "tableRow",
    "tableHeader",
    "tableCell",
    "panel",
    "expand",
    "nestedExpand",
    "layoutSection",
    "layoutColumn",
    "mediaSingle",
    "mediaGroup",
    "media",
    "blockCard",
    "embedCard",
    "extension",
    "bodiedExtension",
];

/// All known ADF inline node type names.
pub const INLINE_NODES: &[&str] = &[
    "text",
    "hardBreak",
    "emoji",
    "mention",
    "date",
    "status",
    "inlineCard",
    "mediaInline",
    "placeholder",
];

/// All known ADF mark type names.
pub const MARKS: &[&str] = &[
    "strong",
    "em",
    "code",
    "strike",
    "underline",
    "link",
    "subsup",
    "textColor",
    "backgroundColor",
    "annotation",
    "border",
];
