#!/usr/bin/env python3
"""Tests for release archive validation and final-notes rendering."""

from __future__ import annotations

import hashlib
import json
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from render_release_notes import render
from verify_artifacts import verify_artifacts


COMMIT = "0123456789abcdef0123456789abcdef01234567"
VERSION = "0.1.0"
RUSTC = "rustc 1.99.1 (58cf8f9c3 2026-10-13)"
CARGO = "cargo 1.99.1 (5f94df478 2026-08-27)"


class ReleaseToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.template = self.root / "template.md"
        self.template.write_text(
            "<!-- template -->\n# SignerReach v0.1.0\n\n"
            "Commit `{{SOURCE_COMMIT}}`.\n\n{{ARTIFACT_TABLE}}\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_package(self, target: str, runner: str, suffix: str, binary: str) -> None:
        package = f"signerreach-v{VERSION}-{target}"
        directory = self.root / target
        directory.mkdir()
        files = {
            "LICENSE": b"Apache-2.0\n",
            "README.md": b"SignerReach\n",
            binary: b"binary\n",
        }
        archive = directory / f"{package}{suffix}"
        if suffix == ".zip":
            with zipfile.ZipFile(archive, "w") as source:
                for name, data in files.items():
                    source.writestr(f"{package}/{name}", data)
        else:
            stage = self.root / f"stage-{target}" / package
            stage.mkdir(parents=True)
            for name, data in files.items():
                (stage / name).write_bytes(data)
            with tarfile.open(archive, "w:gz") as source:
                source.add(stage, arcname=package)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (directory / f"{package}.sha256").write_text(
            f"{digest}  {archive.name}\n", encoding="ascii"
        )
        (directory / f"{package}.manifest.json").write_text(
            json.dumps(
                {
                    "version": VERSION,
                    "source_commit": COMMIT,
                    "runner": runner,
                    "target": target,
                    "rustc": RUSTC,
                    "cargo": CARGO,
                    "archive": archive.name,
                    "sha256": digest,
                }
            ),
            encoding="utf-8",
        )

    def write_valid_packages(self) -> None:
        self.write_package("x86_64-unknown-linux-gnu", "Linux", ".tar.gz", "signerreach")
        self.write_package("aarch64-apple-darwin", "macOS", ".tar.gz", "signerreach")
        self.write_package("x86_64-pc-windows-msvc", "Windows", ".zip", "signerreach.exe")

    def test_validates_all_native_packages(self) -> None:
        self.write_valid_packages()
        result = verify_artifacts(self.root, VERSION, COMMIT)
        self.assertEqual([item["target"] for item in result], [
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ])

    def test_rejects_manifest_source_mismatch(self) -> None:
        self.write_valid_packages()
        manifest = next(self.root.rglob("*.manifest.json"))
        data = json.loads(manifest.read_text(encoding="utf-8"))
        data["source_commit"] = "f" * 40
        manifest.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaises(ValueError):
            verify_artifacts(self.root, VERSION, COMMIT)

    def test_renders_final_notes_without_draft_language(self) -> None:
        self.write_valid_packages()
        output = self.root / "release-notes.md"
        render(self.template, self.root, output, VERSION, COMMIT)
        text = output.read_text(encoding="utf-8")
        self.assertIn(COMMIT, text)
        self.assertIn("x86_64-pc-windows-msvc", text)
        self.assertNotIn("template", text)


if __name__ == "__main__":
    unittest.main()
