//! ADF (Atlassian Document Format) type model.
//!
//! Every ADF node and mark is represented as a strongly-typed Rust enum with
//! serde tags matching the JSON wire format used by Confluence Cloud.

use serde::{Deserialize, Serialize};

// ── Document ────────────────────────────────────────────────────────────────

/// Top-level ADF document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    #[serde(rename = "type")]
    pub doc_type: DocType,
    pub content: Vec<Node>,
}

/// The document type tag – always "doc".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocType {
    #[serde(rename = "doc")]
    Doc,
}

impl Document {
    pub fn new(content: Vec<Node>) -> Self {
        Self {
            version: 1,
            doc_type: DocType::Doc,
            content,
        }
    }
}

// ── Nodes ───────────────────────────────────────────────────────────────────

/// Every ADF block or inline node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    // Block nodes
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "heading")]
    Heading {
        attrs: HeadingAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "bulletList")]
    BulletList { content: Vec<Node> },

    #[serde(rename = "orderedList")]
    OrderedList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<OrderedListAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "listItem")]
    ListItem { content: Vec<Node> },

    #[serde(rename = "taskList")]
    TaskList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TaskListAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "taskItem")]
    TaskItem {
        attrs: TaskItemAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "decisionList")]
    DecisionList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<DecisionListAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "decisionItem")]
    DecisionItem {
        attrs: DecisionItemAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "blockquote")]
    Blockquote { content: Vec<Node> },

    #[serde(rename = "codeBlock")]
    CodeBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<CodeBlockAttrs>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "rule")]
    Rule,

    #[serde(rename = "table")]
    Table {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "tableRow")]
    TableRow { content: Vec<Node> },

    #[serde(rename = "tableHeader")]
    TableHeader {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableCellAttrs>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "tableCell")]
    TableCell {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<TableCellAttrs>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "panel")]
    Panel {
        attrs: PanelAttrs,
        content: Vec<Node>,
    },

    #[serde(rename = "expand")]
    Expand {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<ExpandAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "nestedExpand")]
    NestedExpand {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<ExpandAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "layoutSection")]
    LayoutSection { content: Vec<Node> },

    #[serde(rename = "layoutColumn")]
    LayoutColumn {
        attrs: LayoutColumnAttrs,
        content: Vec<Node>,
    },

    #[serde(rename = "mediaSingle")]
    MediaSingle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attrs: Option<MediaSingleAttrs>,
        content: Vec<Node>,
    },

    #[serde(rename = "mediaGroup")]
    MediaGroup { content: Vec<Node> },

    #[serde(rename = "media")]
    Media { attrs: MediaAttrs },

    #[serde(rename = "mediaInline")]
    MediaInline { attrs: MediaAttrs },

    #[serde(rename = "blockCard")]
    BlockCard { attrs: CardAttrs },

    #[serde(rename = "embedCard")]
    EmbedCard { attrs: EmbedCardAttrs },

    #[serde(rename = "extension")]
    Extension {
        attrs: ExtensionAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Node>,
    },

    #[serde(rename = "bodiedExtension")]
    BodiedExtension {
        attrs: ExtensionAttrs,
        content: Vec<Node>,
    },

    // Inline nodes
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
    },

    #[serde(rename = "hardBreak")]
    HardBreak,

    #[serde(rename = "emoji")]
    Emoji { attrs: EmojiAttrs },

    #[serde(rename = "mention")]
    Mention { attrs: MentionAttrs },

    #[serde(rename = "date")]
    Date { attrs: DateAttrs },

    #[serde(rename = "status")]
    Status { attrs: StatusAttrs },

    #[serde(rename = "inlineCard")]
    InlineCard { attrs: CardAttrs },

    #[serde(rename = "placeholder")]
    Placeholder { attrs: PlaceholderAttrs },

    /// Catch-all for unknown/future node types — preserves the raw JSON.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

// ── Marks ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Mark {
    #[serde(rename = "strong")]
    Strong,

    #[serde(rename = "em")]
    Em,

    #[serde(rename = "code")]
    Code,

    #[serde(rename = "strike")]
    Strike,

    #[serde(rename = "underline")]
    Underline,

    #[serde(rename = "link")]
    Link { attrs: LinkAttrs },

    #[serde(rename = "subsup")]
    SubSup { attrs: SubSupAttrs },

    #[serde(rename = "textColor")]
    TextColor { attrs: TextColorAttrs },

    #[serde(rename = "backgroundColor")]
    BackgroundColor { attrs: BackgroundColorAttrs },

    #[serde(rename = "annotation")]
    Annotation { attrs: AnnotationAttrs },

    #[serde(rename = "border")]
    Border { attrs: BorderAttrs },

    /// Catch-all for unknown marks.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

// ── Attribute structs ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadingAttrs {
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderedListAttrs {
    #[serde(default = "default_order")]
    pub order: u32,
}

fn default_order() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskListAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskItemAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
    pub state: TaskState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskState {
    Todo,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionListAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionItemAttrs {
    #[serde(rename = "localId")]
    pub local_id: String,
    #[serde(default)]
    pub state: DecisionState,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DecisionState {
    #[default]
    Decided,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeBlockAttrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableAttrs {
    #[serde(rename = "isNumberColumnEnabled", default)]
    pub is_number_column_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCellAttrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colspan: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rowspan: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(rename = "colwidth", default, skip_serializing_if = "Option::is_none")]
    pub col_width: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelAttrs {
    #[serde(rename = "panelType")]
    pub panel_type: PanelType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelType {
    Info,
    Note,
    Warning,
    Success,
    Error,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpandAttrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutColumnAttrs {
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaSingleAttrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaAttrs {
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub id: String,
    pub collection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    File,
    Link,
    External,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardAttrs {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbedCardAttrs {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionAttrs {
    #[serde(rename = "extensionType")]
    pub extension_type: String,
    #[serde(rename = "extensionKey")]
    pub extension_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(rename = "localId", default, skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmojiAttrs {
    #[serde(rename = "shortName")]
    pub short_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentionAttrs {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(
        rename = "accessLevel",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub access_level: Option<String>,
    #[serde(rename = "userType", default, skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateAttrs {
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusAttrs {
    pub text: String,
    pub color: StatusColor,
    #[serde(rename = "localId", default, skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusColor {
    Neutral,
    Purple,
    Blue,
    Red,
    Yellow,
    Green,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceholderAttrs {
    pub text: String,
}

// ── Mark attribute structs ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkAttrs {
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "occurenceKey",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub occurence_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubSupAttrs {
    #[serde(rename = "type")]
    pub sub_sup_type: SubSupType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubSupType {
    Sub,
    Sup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextColorAttrs {
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundColorAttrs {
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotationAttrs {
    pub id: String,
    #[serde(rename = "annotationType")]
    pub annotation_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BorderAttrs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}
