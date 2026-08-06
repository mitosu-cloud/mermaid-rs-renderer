#!/usr/bin/env python3
"""Find and fill the next <DESCRIPTION> placeholder in a Mermaid JSONL dataset."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
from typing import Any, Iterable


DEFAULT_PLACEHOLDER = "<DESCRIPTION>"


def iter_content_fields(value: Any, path: tuple[Any, ...] = ()) -> Iterable[tuple[tuple[Any, ...], dict[str, Any]]]:
    if isinstance(value, dict):
        if isinstance(value.get("content"), str):
            yield path + ("content",), value
        for key, child in value.items():
            yield from iter_content_fields(child, path + (key,))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from iter_content_fields(child, path + (index,))


def iter_assistant_fields(value: Any, path: tuple[Any, ...] = ()) -> Iterable[tuple[tuple[Any, ...], str]]:
    if isinstance(value, dict):
        if value.get("role") == "assistant" and isinstance(value.get("content"), str):
            yield path + ("content",), value["content"]
        for key, child in value.items():
            yield from iter_assistant_fields(child, path + (key,))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from iter_assistant_fields(child, path + (index,))


def format_path(path: tuple[Any, ...]) -> str:
    parts: list[str] = []
    for part in path:
        if isinstance(part, int):
            parts.append(f"[{part}]")
        elif parts:
            parts.append(f".{part}")
        else:
            parts.append(str(part))
    return "".join(parts)


def set_path(root: Any, path: tuple[Any, ...], value: Any) -> None:
    target = root
    for part in path[:-1]:
        target = target[part]
    target[path[-1]] = value


def line_ending(line: str) -> str:
    if line.endswith("\r\n"):
        return "\r\n"
    if line.endswith("\n"):
        return "\n"
    return ""


def find_target(lines: list[str], placeholder: str) -> tuple[int, Any, tuple[Any, ...], str, tuple[Any, ...], str]:
    for index, line in enumerate(lines):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as exc:
            raise SystemExit(f"Invalid JSON on line {index + 1}: {exc}") from exc

        placeholder_fields = [
            (path, holder["content"])
            for path, holder in iter_content_fields(entry)
            if placeholder in holder["content"]
        ]
        if not placeholder_fields:
            continue

        assistant_fields = list(iter_assistant_fields(entry))
        if not assistant_fields:
            raise SystemExit(f"Line {index + 1} has {placeholder!r} but no assistant content field.")

        placeholder_path, placeholder_text = placeholder_fields[0]
        assistant_path, assistant_text = assistant_fields[0]
        return index, entry, placeholder_path, placeholder_text, assistant_path, assistant_text

    raise SystemExit(f"No JSONL entry contains {placeholder!r}.")


def read_description(args: argparse.Namespace) -> str | None:
    if args.description_file:
        return args.description_file.read_text(encoding="utf-8").strip()
    if args.description is not None:
        return args.description.strip()
    return None


def write_updated_file(path: Path, lines: list[str]) -> None:
    with tempfile.NamedTemporaryFile(
        "w",
        dir=path.parent,
        delete=False,
        encoding="utf-8",
        newline="",
    ) as handle:
        temp_name = handle.name
        handle.writelines(lines)
    os.replace(temp_name, path)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Find the next JSONL entry with <DESCRIPTION> and optionally replace it.",
    )
    parser.add_argument("jsonl_path", type=Path, help="Path to the JSONL file to inspect or update.")
    parser.add_argument("--placeholder", default=DEFAULT_PLACEHOLDER, help="Placeholder text to find.")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--description", help="Replacement description.")
    group.add_argument("--description-file", type=Path, help="File containing the replacement description.")
    parser.add_argument("--backup", action="store_true", help="Write a .bak copy before updating.")
    args = parser.parse_args()

    path = args.jsonl_path
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    original_lines = list(lines)
    (
        index,
        entry,
        placeholder_path,
        placeholder_text,
        assistant_path,
        assistant_text,
    ) = find_target(lines, args.placeholder)

    print(f"line: {index + 1}")
    print(f"placeholder_path: {format_path(placeholder_path)}")
    print(f"assistant_content_path: {format_path(assistant_path)}")
    print("assistant_content:")
    print(assistant_text)

    description = read_description(args)
    if description is None:
        return 0
    if not description:
        raise SystemExit("Description is empty.")

    updated_text = placeholder_text.replace(args.placeholder, description, 1)
    set_path(entry, placeholder_path, updated_text)
    updated_line = json.dumps(entry, ensure_ascii=False, separators=(",", ":"))
    lines[index] = updated_line + line_ending(lines[index])

    if args.backup:
        backup_path = path.with_suffix(path.suffix + ".bak")
        backup_path.write_text("".join(original_lines), encoding="utf-8")

    write_updated_file(path, lines)
    print(f"updated_line: {index + 1}")
    print(f"replacement: {description}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
