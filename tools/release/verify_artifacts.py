#!/usr/bin/env python3
"""Verify the native packages used by the publication workflow."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import tarfile
import zipfile
from pathlib import Path
from typing import Any


TARGETS = {
    "x86_64-unknown-linux-gnu": ("Linux", ".tar.gz", "signerreach"),
    "aarch64-apple-darwin": ("macOS", ".tar.gz", "signerreach"),
    "x86_64-pc-windows-msvc": ("Windows", ".zip", "signerreach.exe"),
}
TOOLCHAIN_PATTERN = re.compile(
    r"^(?:rustc|cargo) \d+\.\d+\.\d+ \([0-9a-f]+ \d{4}-\d{2}-\d{2}\)$"
)
COMMIT_PATTERN = re.compile(r"^[0-9a-f]{40}$")


def _error(message: str) -> ValueError:
    return ValueError(message)


def _archive_files(archive: Path, version: str, target: str) -> set[str]:
    """Return regular-file members and reject unexpected archive entries."""

    package = f"signerreach-v{version}-{target}"
    expected = {
        f"{package}/LICENSE",
        f"{package}/README.md",
        f"{package}/{TARGETS[target][2]}",
    }

    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as source:
            names = source.namelist()
        files = {name.rstrip("/") for name in names if name and not name.endswith("/")}
    else:
        with tarfile.open(archive, "r:gz") as source:
            members = source.getmembers()
        files = set()
        for member in members:
            if member.isdir():
                if member.name != package:
                    raise _error(f"{archive.name}: unexpected directory {member.name}")
            elif member.isfile():
                files.add(member.name)
            else:
                raise _error(f"{archive.name}: non-regular archive member {member.name}")

    if files != expected:
        raise _error(
            f"{archive.name}: expected files {sorted(expected)}, got {sorted(files)}"
        )
    return files


def verify_artifacts(root: Path, version: str, source_commit: str) -> list[dict[str, Any]]:
    """Verify all native package manifests, checksums and archive contents."""

    if not COMMIT_PATTERN.fullmatch(source_commit):
        raise _error(f"source commit is not a full lowercase SHA: {source_commit}")

    manifests = sorted(root.rglob("*.manifest.json"))
    if len(manifests) != len(TARGETS):
        raise _error(f"expected {len(TARGETS)} manifests below {root}, got {len(manifests)}")

    verified: list[dict[str, Any]] = []
    seen_targets: set[str] = set()
    for manifest_path in manifests:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        target = manifest.get("target")
        if target not in TARGETS:
            raise _error(f"{manifest_path}: unexpected target {target!r}")
        if target in seen_targets:
            raise _error(f"duplicate manifest for target {target}")
        seen_targets.add(target)

        runner, suffix, binary = TARGETS[target]
        package = f"signerreach-v{version}-{target}"
        expected_archive = f"{package}{suffix}"
        if manifest.get("version") != version:
            raise _error(f"{manifest_path}: version does not equal {version}")
        if manifest.get("source_commit") != source_commit:
            raise _error(f"{manifest_path}: source commit does not equal {source_commit}")
        if manifest.get("runner") != runner:
            raise _error(f"{manifest_path}: runner does not match {runner}")
        if manifest.get("archive") != expected_archive:
            raise _error(f"{manifest_path}: archive name does not match {expected_archive}")
        if not TOOLCHAIN_PATTERN.fullmatch(str(manifest.get("rustc", ""))):
            raise _error(f"{manifest_path}: invalid rustc record")
        if not TOOLCHAIN_PATTERN.fullmatch(str(manifest.get("cargo", ""))):
            raise _error(f"{manifest_path}: invalid cargo record")

        archive = manifest_path.parent / expected_archive
        checksum_path = manifest_path.parent / f"{package}.sha256"
        if not archive.is_file() or not checksum_path.is_file():
            raise _error(f"{manifest_path}: archive or checksum file is missing")

        checksum_lines = checksum_path.read_text(encoding="ascii").splitlines()
        if len(checksum_lines) != 1:
            raise _error(f"{checksum_path}: expected one checksum line")
        checksum_parts = checksum_lines[0].split("  ")
        if len(checksum_parts) != 2 or checksum_parts[1] != expected_archive:
            raise _error(f"{checksum_path}: checksum filename is incorrect")
        checksum = checksum_parts[0]
        if not re.fullmatch(r"[0-9a-f]{64}", checksum):
            raise _error(f"{checksum_path}: invalid SHA-256 value")

        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if digest != checksum or digest != manifest.get("sha256"):
            raise _error(f"{archive}: checksum does not match its manifest")
        _archive_files(archive, version, target)

        verified.append(
            {
                "target": target,
                "runner": runner,
                "archive": expected_archive,
                "sha256": digest,
                "rustc": manifest["rustc"],
                "cargo": manifest["cargo"],
                "manifest": manifest_path,
                "archive_path": archive,
                "checksum_path": checksum_path,
            }
        )

    if seen_targets != set(TARGETS):
        raise _error(f"missing targets: {sorted(set(TARGETS) - seen_targets)}")
    return [next(item for item in verified if item["target"] == target) for target in TARGETS]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()

    for item in verify_artifacts(args.root, args.version, args.source_commit):
        print(
            f"verified {item['target']} {item['archive']} "
            f"{item['sha256']} source={args.source_commit}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
