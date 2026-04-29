//! Round-trip tests: MD → ADF → MD and ADF JSON → Document → ADF JSON.
//!
//! For each fixture pair (*.md + *.adf.json), we test:
//! 1. md_to_adf produces the expected ADF JSON
//! 2. adf_to_md on the expected ADF produces valid Markdown
//! 3. The round-trip md → adf → md → adf is stable (second ADF matches first)

use md2conf2md::{adf_json_to_md, adf_to_md, md_to_adf, md_to_adf_json};
use pretty_assertions::assert_eq;

/// Helper: normalize JSON by parsing and re-serializing to remove whitespace differences.
fn normalize_json(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("valid JSON")
}

/// Test MD → ADF for a fixture.
fn test_md_to_adf(name: &str) {
    let md = std::fs::read_to_string(format!("tests/fixtures/{name}.md"))
        .unwrap_or_else(|_| panic!("missing fixture {name}.md"));
    let expected_json = std::fs::read_to_string(format!("tests/fixtures/{name}.adf.json"))
        .unwrap_or_else(|_| panic!("missing fixture {name}.adf.json"));

    let result_json = md_to_adf_json(&md).expect("md_to_adf_json should succeed");
    let result_val = normalize_json(&result_json);
    let expected_val = normalize_json(&expected_json);

    assert_eq!(result_val, expected_val, "MD → ADF mismatch for {name}");
}

/// Test ADF → MD → ADF stability for a fixture.
fn test_adf_roundtrip(name: &str) {
    let expected_json = std::fs::read_to_string(format!("tests/fixtures/{name}.adf.json"))
        .unwrap_or_else(|_| panic!("missing fixture {name}.adf.json"));

    // ADF → MD
    let md = adf_json_to_md(&expected_json).expect("adf_json_to_md should succeed");
    assert!(!md.is_empty(), "ADF → MD produced empty output for {name}");

    // MD → ADF (round-trip)
    let roundtrip_json = md_to_adf_json(&md).expect("roundtrip md_to_adf_json should succeed");
    let roundtrip_val = normalize_json(&roundtrip_json);
    let expected_val = normalize_json(&expected_json);

    assert_eq!(
        roundtrip_val, expected_val,
        "ADF → MD → ADF round-trip mismatch for {name}\nIntermediate MD:\n{md}"
    );
}

/// Test that md_to_adf and adf_to_md are inverse operations at the MD level:
/// md → adf → md → adf should be stable after the first conversion.
fn test_md_stability(name: &str) {
    let md = std::fs::read_to_string(format!("tests/fixtures/{name}.md"))
        .unwrap_or_else(|_| panic!("missing fixture {name}.md"));

    let doc1 = md_to_adf(&md).expect("first md_to_adf");
    let md2 = adf_to_md(&doc1).expect("first adf_to_md");
    let doc2 = md_to_adf(&md2).expect("second md_to_adf");
    let md3 = adf_to_md(&doc2).expect("second adf_to_md");

    // After the first round-trip, MD should be stable
    assert_eq!(md2, md3, "MD not stable after round-trip for {name}");
}

// ── MD → ADF fixture tests ─────────────────────────────────────────────────

#[test]
fn md_to_adf_heading() {
    test_md_to_adf("heading");
}

#[test]
fn md_to_adf_paragraph() {
    test_md_to_adf("paragraph");
}

#[test]
fn md_to_adf_inline_marks() {
    test_md_to_adf("inline_marks");
}

#[test]
fn md_to_adf_link() {
    test_md_to_adf("link");
}

#[test]
fn md_to_adf_bullet_list() {
    test_md_to_adf("bullet_list");
}

#[test]
fn md_to_adf_ordered_list() {
    test_md_to_adf("ordered_list");
}

#[test]
fn md_to_adf_code_block() {
    test_md_to_adf("code_block");
}

#[test]
fn md_to_adf_blockquote() {
    test_md_to_adf("blockquote");
}

#[test]
fn md_to_adf_rule() {
    test_md_to_adf("rule");
}

#[test]
fn md_to_adf_table() {
    test_md_to_adf("table");
}

#[test]
fn md_to_adf_task_list() {
    test_md_to_adf("task_list");
}

#[test]
fn md_to_adf_panel() {
    test_md_to_adf("panel");
}

#[test]
fn md_to_adf_expand() {
    test_md_to_adf("expand");
}

#[test]
fn md_to_adf_status() {
    test_md_to_adf("status");
}

#[test]
fn md_to_adf_image() {
    test_md_to_adf("image");
}

#[test]
fn roundtrip_image() {
    test_adf_roundtrip("image");
}

#[test]
fn stability_image() {
    test_md_stability("image");
}

#[test]
fn md_to_adf_nested_list() {
    test_md_to_adf("nested_list");
}

#[test]
fn roundtrip_nested_list() {
    test_adf_roundtrip("nested_list");
}

#[test]
fn stability_nested_list() {
    test_md_stability("nested_list");
}

#[test]
fn md_to_adf_hard_break() {
    test_md_to_adf("hard_break");
}

#[test]
fn roundtrip_hard_break() {
    test_adf_roundtrip("hard_break");
}

#[test]
fn md_to_adf_multipara_list() {
    test_md_to_adf("multipara_list");
}

#[test]
fn roundtrip_multipara_list() {
    test_adf_roundtrip("multipara_list");
}

#[test]
fn md_to_adf_decision_list() {
    test_md_to_adf("decision_list");
}

#[test]
fn roundtrip_decision_list() {
    test_adf_roundtrip("decision_list");
}

#[test]
fn stability_decision_list() {
    test_md_stability("decision_list");
}

#[test]
fn md_to_adf_card() {
    test_md_to_adf("card");
}

#[test]
fn roundtrip_card() {
    test_adf_roundtrip("card");
}

#[test]
fn stability_card() {
    test_md_stability("card");
}

#[test]
fn md_to_adf_date() {
    test_md_to_adf("date");
}

#[test]
fn roundtrip_date() {
    test_adf_roundtrip("date");
}

#[test]
fn stability_date() {
    test_md_stability("date");
}

#[test]
fn md_to_adf_emoji() {
    test_md_to_adf("emoji");
}

#[test]
fn roundtrip_emoji() {
    test_adf_roundtrip("emoji");
}

#[test]
fn stability_emoji() {
    test_md_stability("emoji");
}

#[test]
fn md_to_adf_mention() {
    test_md_to_adf("mention");
}

#[test]
fn roundtrip_mention() {
    test_adf_roundtrip("mention");
}

#[test]
fn stability_mention() {
    test_md_stability("mention");
}

#[test]
fn md_to_adf_placeholder() {
    test_md_to_adf("placeholder");
}

#[test]
fn roundtrip_placeholder() {
    test_adf_roundtrip("placeholder");
}

#[test]
fn stability_placeholder() {
    test_md_stability("placeholder");
}

#[test]
fn md_to_adf_media_group() {
    test_md_to_adf("media_group");
}

#[test]
fn roundtrip_media_group() {
    test_adf_roundtrip("media_group");
}

#[test]
fn stability_media_group() {
    test_md_stability("media_group");
}

#[test]
fn md_to_adf_media_inline() {
    test_md_to_adf("media_inline");
}

#[test]
fn roundtrip_media_inline() {
    test_adf_roundtrip("media_inline");
}

#[test]
fn stability_media_inline() {
    test_md_stability("media_inline");
}

#[test]
fn md_to_adf_nested_expand() {
    test_md_to_adf("nested_expand");
}

#[test]
fn roundtrip_nested_expand() {
    test_adf_roundtrip("nested_expand");
}

#[test]
fn stability_nested_expand() {
    test_md_stability("nested_expand");
}

#[test]
fn md_to_adf_layout() {
    test_md_to_adf("layout");
}

#[test]
fn roundtrip_layout() {
    test_adf_roundtrip("layout");
}

#[test]
fn stability_layout() {
    test_md_stability("layout");
}

#[test]
fn md_to_adf_extension() {
    test_md_to_adf("extension");
}

#[test]
fn roundtrip_extension() {
    test_adf_roundtrip("extension");
}

#[test]
fn stability_extension() {
    test_md_stability("extension");
}

#[test]
fn md_to_adf_bodied_extension() {
    test_md_to_adf("bodied_extension");
}

#[test]
fn roundtrip_bodied_extension() {
    test_adf_roundtrip("bodied_extension");
}

#[test]
fn stability_bodied_extension() {
    test_md_stability("bodied_extension");
}

#[test]
fn md_to_adf_extra_marks() {
    test_md_to_adf("extra_marks");
}

#[test]
fn roundtrip_extra_marks() {
    test_adf_roundtrip("extra_marks");
}

#[test]
fn stability_extra_marks() {
    test_md_stability("extra_marks");
}

// ── ADF round-trip tests ────────────────────────────────────────────────────

#[test]
fn roundtrip_heading() {
    test_adf_roundtrip("heading");
}

#[test]
fn roundtrip_paragraph() {
    test_adf_roundtrip("paragraph");
}

#[test]
fn roundtrip_inline_marks() {
    test_adf_roundtrip("inline_marks");
}

#[test]
fn roundtrip_bullet_list() {
    test_adf_roundtrip("bullet_list");
}

#[test]
fn roundtrip_ordered_list() {
    test_adf_roundtrip("ordered_list");
}

#[test]
fn roundtrip_code_block() {
    test_adf_roundtrip("code_block");
}

#[test]
fn roundtrip_blockquote() {
    test_adf_roundtrip("blockquote");
}

#[test]
fn roundtrip_rule() {
    test_adf_roundtrip("rule");
}

// ── MD stability tests ─────────────────────────────────────────────────────

#[test]
fn stability_heading() {
    test_md_stability("heading");
}

#[test]
fn stability_paragraph() {
    test_md_stability("paragraph");
}

#[test]
fn stability_bullet_list() {
    test_md_stability("bullet_list");
}

#[test]
fn stability_ordered_list() {
    test_md_stability("ordered_list");
}

#[test]
fn stability_code_block() {
    test_md_stability("code_block");
}

#[test]
fn stability_rule() {
    test_md_stability("rule");
}
