"""End-to-end tests exercising the PyO3 Python bindings.

Run after `maturin develop`:
    pytest tests/test_python.py
"""

import json

import pytest

md2conf2md = pytest.importorskip("md2conf2md")


def test_md_to_adf_returns_dict():
    doc = md2conf2md.md_to_adf("# Title\n\nBody.\n")
    assert isinstance(doc, dict)
    assert doc["type"] == "doc"
    assert doc["version"] == 1
    content = doc["content"]
    assert content[0]["type"] == "heading"
    assert content[0]["attrs"]["level"] == 1
    assert content[1]["type"] == "paragraph"


def test_adf_to_md_accepts_dict():
    doc = {
        "version": 1,
        "type": "doc",
        "content": [
            {
                "type": "paragraph",
                "content": [{"type": "text", "text": "hello"}],
            }
        ],
    }
    md = md2conf2md.adf_to_md(doc)
    assert "hello" in md


def test_md_to_adf_json_is_valid_json():
    s = md2conf2md.md_to_adf_json("Plain paragraph.\n")
    parsed = json.loads(s)
    assert parsed["type"] == "doc"


def test_adf_json_to_md_roundtrip():
    md_in = "# Hello\n\nWorld.\n"
    adf_json = md2conf2md.md_to_adf_json(md_in)
    md_out = md2conf2md.adf_json_to_md(adf_json)
    assert "# Hello" in md_out
    assert "World." in md_out


def test_invalid_adf_json_raises():
    with pytest.raises(md2conf2md.ConversionError):
        md2conf2md.adf_json_to_md("{not valid json")


def test_invalid_adf_dict_raises():
    with pytest.raises(md2conf2md.ConversionError):
        md2conf2md.adf_to_md({"not": "an adf doc"})


def test_conversion_error_is_value_error():
    # ConversionError should be a subclass of ValueError so existing
    # `except ValueError:` handlers keep working.
    assert issubclass(md2conf2md.ConversionError, ValueError)


def test_stability_md_adf_md_adf():
    md = "Some **bold** and *italic* and `code`.\n"
    doc1 = md2conf2md.md_to_adf(md)
    md2 = md2conf2md.adf_to_md(doc1)
    doc2 = md2conf2md.md_to_adf(md2)
    md3 = md2conf2md.adf_to_md(doc2)
    assert md2 == md3
