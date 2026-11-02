#!/usr/bin/env python3
"""Local source and generated-HTML link checks for the SignerReach mdBook.

Uses Python's standard library only. External URLs intentionally remain unchecked.
"""

from __future__ import annotations

import argparse
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

MARKDOWN_LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")


class Links(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.targets: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attr_map = dict(attrs)
        for name in ("href", "src"):
            if attr_map.get(name):
                self.targets.append(attr_map[name] or "")


def check_source(book: Path) -> list[str]:
    errors: list[str] = []
    src = book / "src"
    summary = src / "SUMMARY.md"
    if not summary.is_file():
        return [f"Missing {summary}"]
    book_config = book / "book.toml"
    if not book_config.is_file():
        errors.append(f"Missing {book_config}")
    css = book / "theme" / "signerreach.css"
    if not css.is_file():
        errors.append(f"Missing {css}")

    chapter_names: set[str] = set()
    for match in MARKDOWN_LINK.findall(summary.read_text(encoding="utf-8")):
        path = urlsplit(match.strip().strip("<> ")).path
        if not path.endswith(".md"):
            errors.append(f"SUMMARY has a non-Markdown chapter: {match}")
            continue
        chapter = (src / unquote(path)).resolve()
        if not chapter.is_relative_to(src.resolve()) or not chapter.is_file():
            errors.append(f"Missing or escaping SUMMARY chapter: {path}")
        else:
            chapter_names.add(chapter.name)
    if len(chapter_names) < 7:
        errors.append("SUMMARY needs at least seven substantive chapters")
    for md in sorted(src.rglob("*.md")):
        text = md.read_text(encoding="utf-8")
        for target in MARKDOWN_LINK.findall(text):
            target = target.strip().strip("<> ")
            parsed = urlsplit(target)
            if parsed.scheme or parsed.netloc or target.startswith("#"):
                continue
            rel = unquote(parsed.path)
            if not rel:
                continue
            link = (md.parent / rel).resolve()
            if not link.is_relative_to(src.resolve()) or not link.exists():
                errors.append(f"{md.relative_to(book)}: broken local link {target}")
    return errors


def check_generated_links(site: Path, pages: list[Path] | None = None) -> list[str]:
    """Check generated HTML links and fragments independently of build metadata."""
    site = site.resolve()
    pages = (
        [path.resolve() for path in pages]
        if pages is not None
        else sorted(path.resolve() for path in site.rglob("*.html"))
    )
    ids_by_page = {
        page: set(re.findall(r"\bid=[\"']([^\"']+)[\"']", page.read_text(encoding="utf-8")))
        for page in pages
    }
    errors: list[str] = []

    for page in pages:
        html = page.read_text(encoding="utf-8")
        parser = Links()
        parser.feed(html)
        for raw_target in parser.targets:
            parsed = urlsplit(raw_target)
            if parsed.scheme or parsed.netloc:
                continue
            rel = unquote(parsed.path)
            if not rel:
                candidate = page
            else:
                # mdBook uses site-relative links for its 404 handler; other links are page-relative.
                if rel.startswith("/signerreach/"):
                    candidate = site / rel.removeprefix("/signerreach/")
                elif rel.startswith("/"):
                    continue
                else:
                    candidate = page.parent / rel
                candidate = candidate.resolve()
                if not candidate.is_relative_to(site) or not candidate.exists():
                    errors.append(f"{page.relative_to(site)}: broken generated link {raw_target}")
                    continue
            if parsed.fragment:
                fragment = unquote(parsed.fragment)
                if fragment not in ids_by_page.get(candidate, set()):
                    errors.append(f"{page.relative_to(site)}: missing anchor {raw_target}")
    return errors


def check_output(book: Path) -> list[str]:
    errors: list[str] = []
    site = book / "site"
    if not (site / "index.html").is_file():
        return [f"No built site at {site / 'index.html'}"]
    custom_css = [
        site / "signerreach.css",
        site / "theme" / "signerreach.css",
        *sorted((site / "theme").glob("signerreach-*.css")),
    ]
    custom_css = [path for path in custom_css if path.is_file()]
    if not custom_css:
        errors.append("Custom theme stylesheet was not copied to the site")
    else:
        css = "\n".join(path.read_text(encoding="utf-8") for path in custom_css)
        if "@media (max-width: 600px)" not in css:
            errors.append("Custom theme stylesheet has no mobile breakpoint")
        if ".light" not in css or ".navy" not in css:
            errors.append("Custom theme stylesheet does not define light and dark themes")

    pages = sorted(site.rglob("*.html"))
    if len(pages) < 7:
        errors.append(f"Expected multiple HTML pages, got {len(pages)}")
    for page in pages:
        html = page.read_text(encoding="utf-8")
        if 'name="viewport"' not in html and "name='viewport'" not in html:
            errors.append(f"{page.relative_to(site)}: no responsive viewport metadata")
    errors.extend(check_generated_links(site, pages))

    index = (site / "index.html").read_text(encoding="utf-8")
    if 'id="mdbook-sidebar"' not in index:
        errors.append("index.html: mdBook sidebar navigation is missing")
    if not list(site.glob("searchindex-*.js")) or not list(site.glob("searcher-*.js")):
        errors.append("Generated search assets are missing")
    return errors


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("book", nargs="?", default="book", type=Path)
    ap.add_argument("--source-only", action="store_true")
    args = ap.parse_args()
    errors = check_source(args.book)
    if not args.source_only:
        errors += check_output(args.book)
    if errors:
        for item in errors:
            print(f"ERROR: {item}", file=sys.stderr)
        return 1
    print("SignerReach book checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
