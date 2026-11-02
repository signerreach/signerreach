#!/usr/bin/env python3
"""Render final v0.1.0 notes from a reviewed template and verified artifacts."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

from verify_artifacts import verify_artifacts


FORBIDDEN_PHRASES = (
    "Draft release notes",
    "not a published release",
    "release-preparation PR",
    "review-only artifacts",
    "must be rebuilt",
)


def render(template: Path, artifacts: Path, output: Path, version: str, source_commit: str) -> None:
    text = template.read_text(encoding="utf-8")
    verified = verify_artifacts(artifacts, version, source_commit)
    rows = [
        "| Platform | Target triple | Toolchain | Archive | SHA-256 |",
        "|---|---|---|---|---|",
    ]
    for item in verified:
        rows.append(
            f"| {item['runner']} | `{item['target']}` | "
            f"`{item['rustc']}` / `{item['cargo']}` | "
            f"`{item['archive']}` | `{item['sha256']}` |"
        )
    text = re.sub(r"<!--.*?-->\s*", "", text, flags=re.DOTALL)
    if text.count("{{SOURCE_COMMIT}}") != 1 or text.count("{{ARTIFACT_TABLE}}") != 1:
        raise ValueError("template must contain exactly one source and artifact placeholder")
    rendered = text.replace("{{SOURCE_COMMIT}}", source_commit).replace(
        "{{ARTIFACT_TABLE}}", "\n".join(rows)
    )
    if "{{" in rendered or "}}" in rendered:
        raise ValueError("rendered release notes contain an unresolved placeholder")
    for phrase in FORBIDDEN_PHRASES:
        if phrase.lower() in rendered.lower():
            raise ValueError(f"published release notes contain draft wording: {phrase}")
    output.write_text(rendered.rstrip() + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("template", type=Path)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()
    render(args.template, args.artifacts, args.output, args.version, args.source_commit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
