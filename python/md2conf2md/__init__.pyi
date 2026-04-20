"""Type stubs for md2conf2md."""

from typing import Any

class ConversionError(ValueError):
    """Raised when Markdown ↔ ADF conversion fails."""

def md_to_adf(markdown: str) -> dict[str, Any]:
    """Convert Markdown text to an ADF document (as a Python dict)."""

def adf_to_md(doc: dict[str, Any]) -> str:
    """Convert an ADF document (as a Python dict) to Markdown text."""

def md_to_adf_json(markdown: str) -> str:
    """Convert Markdown text to an ADF JSON string."""

def adf_json_to_md(json: str) -> str:
    """Convert an ADF JSON string to Markdown text."""

__all__ = [
    "ConversionError",
    "md_to_adf",
    "adf_to_md",
    "md_to_adf_json",
    "adf_json_to_md",
]
