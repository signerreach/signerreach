#!/usr/bin/env python3
"""Tests for release archive validation and final-notes rendering."""

from __future__ import annotations

import hashlib
import json
import shutil
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from render_release_notes import render
from release_cleanup import cleanup_plan
from verify_artifacts import verify_artifacts
from verify_release_assets import verify_uploaded_assets


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

    def write_uploaded_assets(self) -> tuple[Path, Path, Path]:
        self.write_valid_packages()
        local_root = self.root / "packages"
        local_root.mkdir()
        downloaded_root = self.root / "downloaded"
        downloaded_root.mkdir()
        metadata: list[dict[str, object]] = []
        suffixes = (".tar.gz", ".zip", ".sha256", ".manifest.json")
        assets = sorted(
            path
            for path in self.root.rglob("*")
            if path.is_file() and path.name.endswith(suffixes)
        )
        for index, source in enumerate(assets, start=1):
            local = local_root / source.name
            shutil.copyfile(source, local)
            downloaded = downloaded_root / source.name
            shutil.copyfile(source, downloaded)
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            metadata.append(
                {
                    "id": index,
                    "name": source.name,
                    "size": source.stat().st_size,
                    "digest": f"sha256:{digest}",
                }
            )
        metadata_path = self.root / "assets.json"
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        return local_root, metadata_path, downloaded_root

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

    def test_rejects_checksum_mismatch(self) -> None:
        self.write_valid_packages()
        checksum = next(self.root.rglob("*.sha256"))
        checksum.write_text("0" * 64 + checksum.read_text(encoding="ascii")[64:], encoding="ascii")
        with self.assertRaises(ValueError):
            verify_artifacts(self.root, VERSION, COMMIT)

    def test_rejects_missing_artifact(self) -> None:
        self.write_valid_packages()
        next(self.root.rglob("*.zip")).unlink()
        with self.assertRaises(ValueError):
            verify_artifacts(self.root, VERSION, COMMIT)

    def test_rejects_extra_artifact(self) -> None:
        self.write_valid_packages()
        (self.root / "extra.manifest.json").write_text("{}", encoding="utf-8")
        with self.assertRaises(ValueError):
            verify_artifacts(self.root, VERSION, COMMIT)

    def test_verifies_uploaded_asset_sizes_and_digests(self) -> None:
        local_root, metadata_path, downloaded_root = self.write_uploaded_assets()
        verified = verify_uploaded_assets(local_root, metadata_path, downloaded_root)
        self.assertEqual(len(verified), 9)

    def test_rejects_uploaded_size_mismatch(self) -> None:
        local_root, metadata_path, downloaded_root = self.write_uploaded_assets()
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata[0]["size"] += 1
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        with self.assertRaises(ValueError):
            verify_uploaded_assets(local_root, metadata_path, downloaded_root)

    def test_rejects_uploaded_digest_mismatch(self) -> None:
        local_root, metadata_path, downloaded_root = self.write_uploaded_assets()
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata[0]["digest"] = "sha256:" + "0" * 64
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        with self.assertRaises(ValueError):
            verify_uploaded_assets(local_root, metadata_path, downloaded_root)

    def test_cleanup_before_tag_creation_deletes_nothing(self) -> None:
        plan = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": None, "owned_release_id": None,
             "tag": {"state": "absent"}, "release": {"state": "absent"}}
        )
        self.assertEqual(plan["delete_release"], False)
        self.assertEqual(plan["delete_tag"], False)

    def test_cleanup_after_tag_before_draft_deletes_only_owned_tag(self) -> None:
        plan = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": None,
             "tag": {"state": "found", "object_type": "tag", "object_sha": "tag-sha"},
             "release": {"state": "absent"}}
        )
        self.assertEqual(plan["delete_release"], False)
        self.assertEqual(plan["delete_tag"], True)

    def test_cleanup_deletes_confirmed_workflow_owned_draft(self) -> None:
        plan = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": "42",
             "tag": {"state": "found", "object_type": "tag", "object_sha": "tag-sha"},
             "release": {"state": "draft", "id": "42", "tag_name": "v0.1.0"}}
        )
        self.assertEqual(plan["delete_release"], True)
        self.assertEqual(plan["delete_tag"], True)

    def test_cleanup_preserves_public_release_and_tag(self) -> None:
        plan = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": "42",
             "tag": {"state": "found", "object_type": "tag", "object_sha": "tag-sha"},
             "release": {"state": "published", "id": "42", "tag_name": "v0.1.0"}}
        )
        self.assertEqual(plan["delete_release"], False)
        self.assertEqual(plan["delete_tag"], False)

    def test_cleanup_preserves_ambiguous_api_state(self) -> None:
        plan = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": "42",
             "tag": {"state": "unknown"}, "release": {"state": "unknown"}}
        )
        self.assertEqual(plan["delete_release"], False)
        self.assertEqual(plan["delete_tag"], False)

    def test_cleanup_preserves_pre_existing_release_or_tag(self) -> None:
        draft_without_workflow_ownership = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": None,
             "tag": {"state": "found", "object_type": "tag", "object_sha": "tag-sha"},
             "release": {"state": "draft", "id": "99", "tag_name": "v0.1.0"}}
        )
        foreign_release = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": None,
             "tag": {"state": "found", "object_type": "tag", "object_sha": "tag-sha"},
             "release": {"state": "foreign", "id": "99", "tag_name": "v0.1.0"}}
        )
        foreign_tag = cleanup_plan(
            {"tag_name": "v0.1.0", "owned_tag_object_sha": "tag-sha", "owned_release_id": "42",
             "tag": {"state": "found", "object_type": "tag", "object_sha": "other-sha"},
             "release": {"state": "draft", "id": "42", "tag_name": "v0.1.0"}}
        )
        self.assertEqual(draft_without_workflow_ownership["delete_release"], False)
        self.assertEqual(draft_without_workflow_ownership["delete_tag"], False)
        self.assertEqual(foreign_release["delete_release"], False)
        self.assertEqual(foreign_release["delete_tag"], False)
        self.assertEqual(foreign_tag["delete_release"], True)
        self.assertEqual(foreign_tag["delete_tag"], False)

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
