# PLAN.md

Implementation plan for md2conf2md — a Rust crate + Python module for bidirectional Markdown ↔ Confluence ADF conversion.

## Current State

The project has **72 passing Rust tests + 8 Python tests**, covering the core conversion pipeline, all Phase 1-3 gaps, and the Python bindings. The Rust crate builds cleanly (`cargo build`), clippy is warning-free on `--all-targets`, and the PyO3 bindings compile via `maturin develop`.

### Phases complete
- **Phase 1 (Correctness):** image alt, footnotes, nested lists, hard breaks (incl. table cells), multi-paragraph list items. Table alignment intentionally dropped (ADF has no such concept).
- **Phase 2 (More nodes):** decision list, inline/block/embed cards, `adf:ext` fence, `:placeholder[…]`, NestedExpand detection for panels/expands/layout columns.
- **Phase 3 (Marks):** link mark rewritten, Code mark ordered innermost, `:span[…]` directive round-trips.
- **Phase 4.2:** markdown metacharacter escaping in plain text.
- **Phase 5:** `ConversionError` exception, `py.typed` marker, `__init__.pyi` stubs, `tests/test_python.py`.
- **Phase 6:** `.github/workflows/ci.yml` for Rust + Python across Linux/macOS/Windows, crate metadata filled in.

### What exists today

**ADF model** (`src/adf/model.rs`): ~25 node variants, 10 mark variants, all attribute structs with serde tags matching the ADF JSON wire format. Includes `Node::Unknown(Value)` catch-all for lossless round-tripping of unrecognized nodes.

**MD → ADF** (`src/md_to_adf/`): Parses Markdown via comrak (GFM mode: tables, task lists, strikethrough, autolinks, footnotes, front matter). Walks the comrak AST to produce `Document`. Post-processes with extension expander for `adf:*` fences and inline directives.

**ADF → MD** (`src/adf_to_md/`): Walks `Document` tree, emits Markdown with correct blockquote nesting, list indentation, GFM tables, and extension microsyntax output.

**Extensions**: Panel, expand, layout fences; inline status, date, mention, emoji directives; `adf:raw` lossless fallback. Both directions.

**PyO3 bindings** (`src/py.rs`): Four functions — `md_to_adf`, `adf_to_md`, `md_to_adf_json`, `adf_json_to_md`. Feature-gated on `python`.

**Tests**: 14 fixture pairs (`tests/fixtures/*.md` + `*.adf.json`), 20 unit tests (`tests/nodes.rs`), 28 roundtrip/stability tests (`tests/roundtrip.rs`), 1 doctest.

### What's missing or incomplete

The items below are ordered by priority. Each section describes what to do and why.

---

## Phase 1: Correctness & Coverage Gaps

These are gaps in the current implementation that could produce incorrect output for real-world Confluence documents.

### 1.1 Image alt text vs title

**File**: `src/md_to_adf/inlines.rs:92-111`

The image handler uses `link.title` for `alt` but comrak separates `alt` (from `![alt](url)`) and `title` (from `![alt](url "title")`). The alt text is in the child text nodes of the Image AST node, not in `link.title`. Fix:
- Walk child text nodes of the Image to extract alt text
- Use `link.title` only for the title attribute if we need it

### 1.2 Footnotes

**Status**: comrak parses footnotes (`[^1]` / `[^1]: ...`) but the converter ignores `NodeValue::FootnoteDefinition` and `NodeValue::FootnoteReference`. ADF has no footnote node.

**Approach**: Convert footnotes to a "References" section at the end of the document with numbered paragraphs and back-links. Or emit them as superscript inline references. Decide on a canonical form and implement both directions.

### 1.3 Nested lists

**Current behavior**: Nested lists work through comrak's AST (ListItem containing List), but the ADF→MD writer's `write_list_item` only indents continuation blocks, not nested lists within list items. Verify and add fixtures for:
- Bullet list inside ordered list
- 3+ levels of nesting
- Task list nested inside bullet list

### 1.4 Hard breaks in ADF→MD

**File**: `src/adf_to_md/inlines.rs:72`

`HardBreak` emits `\\\n` but this inserts a literal newline into the inline text flow, which may break if the parent is still accumulating inline content (e.g., inside a table cell). Verify behavior in all container contexts.

### 1.5 Table column alignment

GFM tables support `:---`, `:---:`, `---:` for left/center/right alignment. comrak parses this into `NodeValue::Table(alignments)` but the MD→ADF converter ignores alignment (ADF tables don't natively support column alignment). The ADF→MD writer always emits ` --- `. Decide whether to:
- Preserve alignment in a custom attribute on the table node
- Emit alignment markers on round-trip even though ADF doesn't use them

### 1.6 Multi-paragraph list items

A list item can contain multiple paragraphs, code blocks, blockquotes, etc. The current `write_list_item` handles the first child specially but may not indent subsequent children correctly for all node types. Add fixtures and verify.

---

## Phase 2: More ADF Node Types

These nodes exist in the model but have incomplete or missing conversion support.

### 2.1 Decision list

**Model**: `DecisionList`, `DecisionItem`, `DecisionState` exist. **MD→ADF**: Not handled in `blocks.rs` (no comrak AST equivalent). **ADF→MD**: Writer exists using `- [?] ` / `- [!] ` syntax.

**TODO**: Add extension parsing for decision list syntax in `md_to_adf/extensions.rs`. Add fixture pair.

### 2.2 Inline card / Block card / Embed card

**Model**: `InlineCard`, `BlockCard`, `EmbedCard` exist. **ADF→MD**: Writer emits `[](url){card=inline}` etc. **MD→ADF**: Extension parser does NOT recognize the `{card=...}` directive syntax on links.

**TODO**: In `md_to_adf/extensions.rs`, add a post-processing pass that recognizes `{card=inline}`, `{card=block}`, `{card=embed}` trailing directives on link/image nodes.

### 2.3 Media with collection/id attributes

**Current**: Images map to `MediaSingle > Media` with `media_type: External` and the URL as `id`. Real Confluence media uses `media_type: File` with a UUID `id` and a `collection` string.

**TODO**: When the image URL looks like a UUID (or is explicitly marked), produce `File` type instead of `External`. Possibly add `{type=file collection=...}` directive support.

### 2.4 Extension / BodiedExtension

**Model**: Exists. **ADF→MD**: Emits `adf:ext` fence. **MD→ADF**: The `adf:ext` fence is not handled in `expand_adf_fence` (falls through to "unknown adf: type" path, preserving as code block).

**TODO**: Add `"ext"` arm in `expand_adf_fence` that constructs `Node::Extension` or `Node::BodiedExtension` from the fence attributes.

### 2.5 Placeholder

**Model**: Exists. **ADF→MD**: Writer emits `:placeholder[text]`. **MD→ADF**: Not parsed.

**TODO**: Add `:placeholder[...]` to inline directive parser.

### 2.6 Nested expand

**ADF→MD**: Handled (shares code with Expand). **MD→ADF**: Always produces `Expand`, never `NestedExpand`. The distinction only matters when an expand is inside another container (panel, another expand, layout column).

**TODO**: In extension expansion, detect when an expand is nested inside another container and produce `NestedExpand` instead.

---

## Phase 3: Mark Handling Improvements

### 3.1 Link mark with title

**ADF→MD** (`src/adf_to_md/inlines.rs:110-121`): The link mark handler has a bug — it writes `](href)` then tries to rewrite with a title by truncating, but the truncation logic is fragile.

**TODO**: Rewrite the link emission to build the complete `[text](href "title")` string in one pass.

### 3.2 Nested marks ordering

When text has multiple marks (e.g., bold + italic + code), the emission order matters for Markdown parsing. Currently marks are emitted in the order they appear in the `marks` array. Code mark should be innermost (closest to text) since backticks don't allow nesting.

**TODO**: Sort marks so `Code` is always last in the prefix and first in the suffix.

### 3.3 :span directive round-trip

**ADF→MD**: Emits `:span[text]{underline=1 color=#ff0000}`. **MD→ADF**: The inline directive parser does NOT handle `:span[...]`. Text with underline, textColor, backgroundColor, subsup, or border marks will lose those marks on round-trip.

**TODO**: Add `:span[...]` parsing in `try_parse_directive` that constructs a `Node::Text` with the appropriate marks.

---

## Phase 4: Robustness & Edge Cases

### 4.1 Empty nodes

Test and handle edge cases:
- Empty paragraph (should produce `{"type":"paragraph","content":[]}` or be omitted?)
- Empty code block (no text node)
- Empty table (no rows)
- Empty list (no items)
- Panel/expand with no content

### 4.2 Special characters in text

Markdown special characters (`*`, `_`, `` ` ``, `[`, etc.) in ADF text nodes need escaping when emitted as Markdown. Currently `adf_to_md` writes text verbatim, which means ADF text like `use *args` would produce accidental italic in the Markdown output.

**TODO**: Add an escaping pass in `write_marked_text` that backslash-escapes Markdown metacharacters in plain text spans.

### 4.3 CDATA and HTML entities in code blocks

ADF code blocks store literal text. When this text contains `<`, `>`, `&`, the JSON is fine but if we ever need to emit CSF (XHTML), these need CDATA wrapping. Not an issue for the current ADF-only scope but worth noting.

### 4.4 Unicode and emoji edge cases

The emoji handler maps `:shortName:` but there's no emoji name validation. A colon-separated word that isn't an emoji would still be treated as one. Consider adding a known-emoji list or only parsing emoji directives with the explicit `:emoji[...]` syntax.

### 4.5 Large documents / performance

comrak is fast but the extension post-processing pass (`expand_extensions`) clones nodes during recursion. For very large documents, consider:
- In-place mutation instead of `into_iter().flat_map().collect()`
- Benchmarks with a 1000-paragraph document

---

## Phase 5: Python Module Polish

### 5.1 Type stubs

Create `python/md2conf2md/py.typed` marker and `python/md2conf2md/__init__.pyi` stub file so type checkers (mypy, pyright) understand the API:

```python
def md_to_adf(markdown: str) -> dict: ...
def adf_to_md(doc: dict) -> str: ...
def md_to_adf_json(markdown: str) -> str: ...
def adf_json_to_md(json: str) -> str: ...
```

### 5.2 Error types

Currently all errors become `ValueError`. Consider defining `md2conf2md.ConversionError` as a custom exception class for better error handling in Python callers.

### 5.3 Python tests

Add `tests/test_python.py` with pytest tests that exercise the Python API directly (not just through Rust). These should run after `maturin develop` and test:
- Basic conversion in both directions
- Error handling (invalid JSON, invalid Markdown)
- Dict structure matches expected ADF schema
- Round-trip stability

### 5.4 Publish to PyPI

Set up `maturin build` in CI and publish wheels. The `pyproject.toml` is already configured for maturin; need to add GitHub Actions workflow for multi-platform wheel builds.

---

## Phase 6: CI & Distribution

### 6.1 GitHub Actions workflow

```yaml
# .github/workflows/ci.yml
- cargo test
- cargo clippy -- -D warnings
- cargo fmt --check
- maturin develop && python -m pytest tests/
```

### 6.2 Crate publish

Publish to crates.io as `md2conf2md`. Ensure `Cargo.toml` has all required metadata (repository, documentation, categories, keywords).

### 6.3 Benchmarks

Add `benches/` directory with criterion benchmarks:
- MD→ADF throughput (paragraphs, tables, mixed content)
- ADF→MD throughput
- Round-trip stability check at scale

---

## Phase 7: Advanced Features (Future)

These are significant features that expand the scope beyond basic conversion.

### 7.1 CSF (Confluence Storage Format) support

The reference repo (`third-party/md2conf-python`) uses CSF (XHTML with `ac:`/`ri:` namespaces), not ADF. Many Confluence Server/Data Center installations use CSF exclusively.

**Approach**: Add `src/csf/` module with:
- CSF XML parser → ADF model (reuse existing model as the hub)
- ADF model → CSF XML emitter
- Public API: `md_to_csf()`, `csf_to_md()`, `csf_to_adf()`, `adf_to_csf()`

This is a large effort — CSF has DTD-defined entities, namespace-qualified attributes, and macro-specific XML structures.

### 7.2 Confluence API client

A thin Rust HTTP client that can fetch/push pages from Confluence Cloud (REST v2, ADF-native) and Server (REST v1, CSF). Not in the converter library itself — possibly a separate crate or binary.

### 7.3 Streaming / incremental conversion

For very large documents or editor integration, support converting a stream of ADF nodes → Markdown chunks without buffering the entire document.

### 7.4 WASM target

Compile to WebAssembly for browser-based conversion (e.g., in a Confluence→Markdown browser extension). comrak supports WASM. PyO3 does not, so this would be the Rust-only API.

---

## Fixture Coverage Matrix

Each row should have a `tests/fixtures/{name}.md` + `tests/fixtures/{name}.adf.json` pair, plus entries in `tests/roundtrip.rs`. Check marks indicate existing fixtures.

| Node/Feature | Fixture exists | MD→ADF test | Round-trip test | Stability test |
|---|---|---|---|---|
| paragraph | yes | yes | yes | yes |
| heading | yes | yes | yes | yes |
| bullet_list | yes | yes | yes | yes |
| ordered_list | yes | yes | yes | yes |
| task_list | yes | yes | - | - |
| code_block | yes | yes | yes | yes |
| blockquote | yes | yes | yes | - |
| rule | yes | yes | yes | yes |
| table | yes | yes | - | - |
| inline_marks | yes | yes | yes | - |
| link | yes | yes | - | - |
| panel | yes | yes | - | - |
| expand | yes | yes | - | - |
| status | yes | yes | - | - |
| decision_list | yes | yes | yes | yes |
| nested_list | yes | yes | yes | yes |
| image | yes | yes | yes | yes |
| card (inline/block/embed) | yes | yes | yes | yes |
| multipara_list | yes | yes | yes | - |
| hard_break | yes | yes | yes | - |
| mention | **no** | **no** | **no** | **no** |
| date | **no** | **no** | **no** | **no** |
| emoji | **no** | **no** | **no** | **no** |
| layout | **no** | **no** | **no** | **no** |
| media_single | covered by image | - | - | - |
| extension | unit test | - | - | - |
| span_marks | unit test | - | - | - |
| footnotes | unit test | - | - | - |
| bold_italic_nested | **no** | **no** | **no** | **no** |
| raw_fallback | unit test | - | - | - |

Priority: Fill in the "no" rows roughly top-to-bottom. Each new fixture validates the corresponding conversion code; if a test fails, the code needs fixing first.
