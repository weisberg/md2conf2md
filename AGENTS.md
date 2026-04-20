# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

**md2conf2md** is a Rust library with Python bindings for bidirectional conversion between Markdown and Confluence ADF (Atlassian Document Format). The core conversion logic is Rust, exposed to Python via PyO3/maturin.

The reference implementation at `/Users/weisberg/Documents/Development/gh/third-party/md2conf-python` provides a mature Markdown→Confluence CSF converter (md2conf) that can be studied for conversion patterns and test fixtures.

## Build & Development Commands

```bash
# Build (Rust only — default)
cargo build

# Run all tests (49 tests: 20 unit, 28 roundtrip, 1 doctest)
cargo test

# Run a single test
cargo test test_name

# Run a single test file
cargo test --test roundtrip
cargo test --test nodes

# Clippy (zero warnings enforced)
cargo clippy

# Format check
cargo fmt --check

# Build Python module (requires maturin: pip install maturin)
maturin develop

# Build Python module with release optimizations
maturin develop --release
```

Note: `cargo build --features python` will fail with linker errors because PyO3 cdylib needs Python symbols. Use `maturin develop` instead for Python builds.

## Architecture

### Conversion Pipeline

```
Markdown text ──→ comrak AST ──→ ADF Document (Rust enums) ──→ ADF JSON
                 (md_to_adf/)         ↕ serde                  (output)
ADF JSON ──→ ADF Document ──→ Markdown text
(input)      (serde)          (adf_to_md/)
```

### Source Layout

- `src/adf/model.rs` — All ADF types: `Document`, `Node` (tagged enum with ~25 variants), `Mark` (10 variants), attribute structs. Serde tags match the ADF JSON wire format. Unknown/future nodes fall through to `Node::Unknown(Value)`.
- `src/adf/schema.rs` — Node/mark name constants.
- `src/md_to_adf/` — Markdown→ADF. Uses comrak (GFM-compatible) to parse into AST, then `blocks.rs` walks block nodes and `inlines.rs` handles inline content with mark accumulation. `extensions.rs` post-processes the result to expand `adf:*` fenced blocks and inline directives (`:status[...]`, `@[...]`, `:date[...]`).
- `src/adf_to_md/` — ADF→Markdown. `blocks.rs` walks the ADF Document tree emitting Markdown. `inlines.rs` handles text with marks. Extensions (panel, expand, layout, etc.) emit `adf:*` fenced blocks.
- `src/py.rs` — PyO3 bindings (feature-gated on `python`). Four functions: `md_to_adf`, `adf_to_md`, `md_to_adf_json`, `adf_json_to_md`.
- `src/lib.rs` — Public Rust API: `md_to_adf()`, `adf_to_md()`, `md_to_adf_json()`, `adf_json_to_md()`.

### Extension Microsyntax

Confluence-only ADF nodes use a standardized Markdown representation:

- **Block extensions**: fenced code blocks with `adf:<type> key=value` info strings (panel, expand, layout, raw)
- **Inline directives**: `:status[text]{color=green}`, `:date[2026-04-20]`, `@[id]{text="Name"}`, `:emoji[name]`
- **Mark directives**: `:span[text]{underline=1 color=#ff0000}` for marks with no CommonMark equivalent
- **Lossless fallback**: `adf:raw` fence wraps unknown node JSON verbatim

### Test Structure

- `tests/nodes.rs` — Per-node-type unit tests for both conversion directions
- `tests/roundtrip.rs` — Fixture-driven tests using paired `tests/fixtures/*.md` + `*.adf.json` files. Tests MD→ADF correctness, ADF→MD→ADF round-trip stability, and MD→ADF→MD stability.

## Key Rust Dependencies

- `comrak` — GFM-compatible Markdown parser (AST-based, not event stream)
- `serde` + `serde_json` — ADF JSON serialization via tagged enums
- `pyo3` + `pythonize` — Python bindings (optional `python` feature)
- `thiserror` — Error types

## Design Decisions

- **comrak over pulldown-cmark**: comrak exposes a mutable AST with 40+ node variants (tables, task items, strikethrough, footnotes) which maps cleanly to ADF's tree structure. pulldown-cmark's event stream requires complex state tracking.
- **ADF model is the hub**: both directions go through strongly-typed `Document`/`Node`/`Mark` enums. Python layer uses `pythonize` for zero-copy serde↔PyObject bridging.
- **`Node::Unknown(Value)` catch-all**: any ADF node type we haven't modeled deserializes as raw JSON and round-trips through `adf:raw` fences. New Atlassian schema additions don't break existing documents.
- **Extension body re-parsing**: content inside `adf:panel`, `adf:expand`, etc. is recursively parsed as Markdown through the full pipeline, so nested formatting works.
