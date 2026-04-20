# md2conf2md

Bidirectional Markdown ↔ Confluence ADF (Atlassian Document Format) converter, written in Rust with Python bindings.

## Features

- **Markdown → ADF**: Convert CommonMark + GFM Markdown to Confluence ADF JSON
- **ADF → Markdown**: Convert ADF JSON back to idiomatic Markdown
- **Round-trip safe**: `md → adf → md → adf` is stable after the first conversion
- **Lossless fallback**: Unknown ADF nodes round-trip via `adf:raw` JSON fences
- **Dual API**: Use as a Rust crate or a Python module

### Supported ADF nodes

| Category | Nodes |
|---|---|
| Block | paragraph, heading (1-6), bulletList, orderedList, taskList, decisionList, blockquote, codeBlock, rule, table |
| Container | panel (info/note/warning/success/error), expand, nestedExpand, layoutSection/layoutColumn |
| Media | mediaSingle, mediaGroup, media, mediaInline, blockCard, embedCard, inlineCard |
| Inline | text, hardBreak, emoji, mention, date, status, placeholder |
| Marks | strong, em, code, strike, underline, link, subsup, textColor, backgroundColor, border |
| Extension | extension, bodiedExtension, unknown (raw JSON fallback) |

## Rust Usage

```rust
let doc = md2conf2md::md_to_adf("# Hello\n\nWorld").unwrap();
let json = md2conf2md::md_to_adf_json("# Hello\n\nWorld").unwrap();
let md = md2conf2md::adf_to_md(&doc).unwrap();
```

## Python Usage

```bash
pip install maturin
maturin develop
```

```python
import md2conf2md

# Markdown → ADF dict
doc = md2conf2md.md_to_adf("# Hello\n\nWorld")

# ADF dict → Markdown
md = md2conf2md.adf_to_md(doc)

# String-in/string-out (skip dict conversion)
adf_json = md2conf2md.md_to_adf_json("# Hello")
md = md2conf2md.adf_json_to_md(adf_json)
```

## Extension Syntax

Confluence-only ADF nodes that have no standard Markdown equivalent use a consistent microsyntax:

### Block extensions (fenced code blocks)

````markdown
```adf:panel type=info
This is an info panel with **rich** content.
```

```adf:expand title="Click to expand"
Hidden content here.
```

```adf:layout widths=50,50
Left column.

---col---

Right column.
```

```adf:raw
{"type":"unknownFutureNode","attrs":{"foo":"bar"}}
```
````

### Inline directives

```markdown
Task is :status[Done]{color=green} and due :date[2026-04-20].
Assigned to @[abc123]{text="Jane Doe"}.
```

## Building

```bash
cargo build          # Rust library
cargo test           # Run all tests
cargo clippy         # Lint
maturin develop      # Build + install Python module
```

## License

MIT
