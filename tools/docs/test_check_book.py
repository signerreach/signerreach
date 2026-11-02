#!/usr/bin/env python3
"""Regression tests for generated documentation link and anchor checks."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from check_book import check_generated_links


class GeneratedAnchorChecks(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.site = Path(self.tempdir.name)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_pages(self, **pages: str) -> None:
        for name, html in pages.items():
            path = self.site / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(html, encoding="utf-8")

    def errors(self) -> list[str]:
        return check_generated_links(self.site)

    def test_valid_same_page_anchor(self) -> None:
        self.write_pages(**{"index.html": "<a href='#welcome'>Welcome</a><h1 id='welcome'>Welcome</h1>"})
        self.assertEqual(self.errors(), [])

    def test_missing_same_page_anchor(self) -> None:
        self.write_pages(**{"index.html": "<a href='#missing'>Missing</a>"})
        errors = self.errors()
        self.assertEqual(len(errors), 1)
        self.assertIn("index.html: missing anchor #missing", errors[0])

    def test_valid_cross_page_anchor(self) -> None:
        self.write_pages(
            **{
                "index.html": "<a href='chapter.html#scope'>Scope</a>",
                "chapter.html": "<h2 id='scope'>Scope</h2>",
            }
        )
        self.assertEqual(self.errors(), [])

    def test_missing_cross_page_anchor(self) -> None:
        self.write_pages(
            **{
                "index.html": "<a href='chapter.html#missing'>Missing</a>",
                "chapter.html": "<h2 id='scope'>Scope</h2>",
            }
        )
        errors = self.errors()
        self.assertEqual(len(errors), 1)
        self.assertIn("index.html: missing anchor chapter.html#missing", errors[0])

    def test_forward_reference_to_later_page_is_order_independent(self) -> None:
        self.write_pages(
            **{
                "index.html": "<a href='later.html#later-heading'>Later</a>",
                "later.html": "<h2 id='later-heading'>Later</h2>",
            }
        )
        self.assertEqual(self.errors(), [])


if __name__ == "__main__":
    unittest.main()
