"""Read knowledge-base.yaml with a stdlib subset parser, so every tool shares one reader."""
# spec: specs/tool-cli.md
# spec-section: Configuration

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

CONFIG_FILE = "knowledge-base.yaml"

_KEY_PATTERN = re.compile(
    r"""^("[^"]*"|'[^']*'|[^\s:#'"\-\[][^:]*?|-[^\s:][^:]*?)\s*:(?:\s+(.*))?$"""
)
_BLOCK_SCALAR = re.compile(r"^[|>][+-]?\d*$")


class ConfigError(ValueError):
    """A configured value has the wrong shape. Tools exit 2 rather than check nothing."""


def _strip_comment(line: str) -> str:
    """Cut a trailing # comment that sits outside quotes."""
    quote = ""
    for i, ch in enumerate(line):
        if quote:
            if ch == quote:
                quote = ""
        elif ch in "\"'":
            quote = ch
        elif ch == "#" and (i == 0 or line[i - 1] in " \t"):
            return line[:i]
    return line


def _unquote(text: str) -> str:
    text = text.strip()
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "\"'":
        return text[1:-1]
    return text


def _split_inline(body: str) -> list[str]:
    """Split the inside of an inline list or mapping on commas outside quotes and brackets."""
    items: list[str] = []
    current = ""
    quote = ""
    depth = 0
    for ch in body:
        if quote:
            current += ch
            if ch == quote:
                quote = ""
        elif ch in "\"'":
            quote = ch
            current += ch
        elif ch in "[{":
            depth += 1
            current += ch
        elif ch in "]}":
            depth -= 1
            current += ch
        elif ch == "," and depth == 0:
            items.append(current)
            current = ""
        else:
            current += ch
    if current.strip():
        items.append(current)
    return [i.strip() for i in items if i.strip()]


def _flow_mapping(body: str) -> dict[str, Any]:
    """An inline mapping ({ by: x, at: 2026-01-01T00:00:00Z }); keys split at the first ": "."""
    result: dict[str, Any] = {}
    for item in _split_inline(body):
        key, sep, value = item.partition(":")
        if not sep:
            continue
        if value and not value[0].isspace():
            # "a:b" is not a key; YAML needs ": ". Retry at the first ": ".
            key, sep, value = item.partition(": ")
            if not sep:
                continue
        result[_unquote(key)] = _scalar(value)
    return result


def _scalar(text: str) -> Any:
    text = text.strip()
    if text.startswith("[") and text.endswith("]"):
        return [_scalar(i) for i in _split_inline(text[1:-1])]
    if text.startswith("{") and text.endswith("}"):
        items = _split_inline(text[1:-1])
        # Only "key: value" items make a mapping; {draft} and {{type}} (a template) stay strings.
        if items and all(re.search(r":(\s|$)", i) for i in items):
            return _flow_mapping(text[1:-1])
    return _unquote(text)


def _logical_lines(text: str) -> list[tuple[int, str]]:
    lines = []
    for raw in text.splitlines():
        cleaned = _strip_comment(raw).rstrip()
        if not cleaned.strip():
            continue
        indent = len(cleaned) - len(cleaned.lstrip(" "))
        lines.append((indent, cleaned.strip()))
    return lines


def _is_item(content: str) -> bool:
    return content == "-" or content.startswith("- ")


def _skip_deeper(lines: list[tuple[int, str]], i: int, indent: int) -> int:
    while i < len(lines) and lines[i][0] > indent:
        i += 1
    return i


def _parse_block(lines: list[tuple[int, str]], i: int, indent: int) -> tuple[Any, int]:
    if _is_item(lines[i][1]):
        return _parse_sequence(lines, i, indent)
    return _parse_mapping(lines, i, indent)


def _parse_mapping(lines: list[tuple[int, str]], i: int, indent: int) -> tuple[dict, int]:
    result: dict[str, Any] = {}
    while i < len(lines) and lines[i][0] == indent and not _is_item(lines[i][1]):
        match = _KEY_PATTERN.match(lines[i][1])
        if not match:
            i += 1  # outside the subset: ignore
            continue
        key = _unquote(match.group(1))
        rest = (match.group(2) or "").strip()
        i += 1
        if rest:
            if _BLOCK_SCALAR.match(rest):
                result[key] = rest
            else:
                result[key] = _scalar(rest)
            i = _skip_deeper(lines, i, indent)
        elif i < len(lines) and (
            lines[i][0] > indent or (lines[i][0] == indent and _is_item(lines[i][1]))
        ):
            result[key], i = _parse_block(lines, i, lines[i][0])
        else:
            result[key] = None
    return result, i


def _parse_sequence(lines: list[tuple[int, str]], i: int, indent: int) -> tuple[list, int]:
    result: list[Any] = []
    while i < len(lines) and lines[i][0] == indent and _is_item(lines[i][1]):
        content = lines[i][1]
        body = content[1:].lstrip()
        offset = len(content) - len(body)
        if not body:
            i += 1
            if i < len(lines) and lines[i][0] > indent:
                value, i = _parse_block(lines, i, lines[i][0])
                result.append(value)
            else:
                result.append(None)
        elif _KEY_PATTERN.match(body) and not body.startswith(("[", "{", '"', "'")):
            # "- path: x": the item is a mapping whose first key shares the dash's line.
            lines[i] = (indent + offset, body)
            value, i = _parse_mapping(lines, i, indent + offset)
            result.append(value)
        else:
            result.append(_scalar(body))
            i = _skip_deeper(lines, i + 1, indent)
    return result, i


def parse_subset(text: str) -> dict[str, Any]:
    """Parse the YAML subset KB configuration uses. Unsupported constructs are ignored."""
    lines = _logical_lines(text)
    if not lines:
        return {}
    value, _ = _parse_block(lines, 0, lines[0][0])
    return value if isinstance(value, dict) else {}


def load_config(root: Path, required: bool = False) -> dict[str, Any]:
    """Read root/knowledge-base.yaml.

    A missing file is an empty configuration, or FileNotFoundError when `required`.
    """
    path = Path(root) / CONFIG_FILE
    if not path.exists():
        if required:
            raise FileNotFoundError(f"{CONFIG_FILE} not found in {root}")
        return {}
    return parse_subset(path.read_text(encoding="utf-8"))


def get_value(config: dict[str, Any], dotted: str) -> Any:
    """Look up a dotted key path ("rules.lifecycle.statuses"); None when absent."""
    node: Any = config
    for part in dotted.split("."):
        if not isinstance(node, dict) or part not in node:
            return None
        node = node[part]
    return node


def get_list(config: dict[str, Any], dotted: str) -> list[str] | None:
    """A list of strings at a key path; None when absent, ConfigError when malformed.

    A lone string reads as a one-item list.
    """
    value = get_value(config, dotted)
    if value is None:
        return None
    if isinstance(value, str):
        return [value]
    if isinstance(value, list) and all(isinstance(v, str) for v in value):
        return list(value)
    raise ConfigError(f"{CONFIG_FILE}: '{dotted}' must be a list of strings")
