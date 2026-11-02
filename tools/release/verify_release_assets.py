#!/usr/bin/env python3
"""Verify the bytes and metadata uploaded to a draft GitHub release."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


ASSET_SUFFIXES = (".tar.gz", ".zip", ".sha256", ".manifest.json")
SHA256_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")


def _error(message: str) -> ValueError:
    return ValueError(message)


def _asset_files(root: Path) -> dict[str, Path]:
    if not root.is_dir():
        raise _error(f"asset directory does not exist: {root}")

    files = [
        path
        for path in root.rglob("*")
        if path.is_file() and path.name.endswith(ASSET_SUFFIXES)
    ]
    names = [path.name for path in files]
    if len(files) != 9:
        raise _error(f"expected 9 local release assets, got {len(files)}")
    if len(set(names)) != len(names):
        raise _error("local release assets contain duplicate filenames")
    return {path.name: path for path in files}


def _digest(path: Path) -> tuple[int, str]:
    data = path.read_bytes()
    return len(data), hashlib.sha256(data).hexdigest()


def verify_uploaded_assets(
    local_root: Path, metadata_path: Path, downloaded_root: Path
) -> list[dict[str, Any]]:
    """Fail closed unless local and downloaded assets match GitHub metadata."""

    local = _asset_files(local_root)
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    if not isinstance(metadata, list):
        raise _error("GitHub asset metadata is not a JSON array")

    metadata_by_name: dict[str, dict[str, Any]] = {}
    for item in metadata:
        if not isinstance(item, dict):
            raise _error("GitHub asset metadata contains a non-object")
        name = item.get("name")
        if not isinstance(name, str) or not name or Path(name).name != name:
            raise _error(f"invalid uploaded asset filename: {name!r}")
        if name in metadata_by_name:
            raise _error(f"duplicate uploaded asset filename: {name}")
        if not isinstance(item.get("id"), int) or item["id"] <= 0:
            raise _error(f"invalid uploaded asset id for {name}")
        if not isinstance(item.get("size"), int) or item["size"] < 0:
            raise _error(f"invalid uploaded asset size for {name}")
        digest = item.get("digest")
        if digest is not None and (
            not isinstance(digest, str) or not SHA256_PATTERN.fullmatch(digest)
        ):
            raise _error(f"invalid uploaded asset digest for {name}")
        metadata_by_name[name] = item

    if set(metadata_by_name) != set(local):
        missing = sorted(set(local) - set(metadata_by_name))
        extra = sorted(set(metadata_by_name) - set(local))
        raise _error(f"uploaded asset names differ; missing={missing}, extra={extra}")

    downloaded = _asset_files(downloaded_root)
    if set(downloaded) != set(local):
        missing = sorted(set(local) - set(downloaded))
        extra = sorted(set(downloaded) - set(local))
        raise _error(f"downloaded asset names differ; missing={missing}, extra={extra}")

    verified: list[dict[str, Any]] = []
    for name in sorted(local):
        local_size, local_digest = _digest(local[name])
        downloaded_size, downloaded_digest = _digest(downloaded[name])
        item = metadata_by_name[name]
        if local_size != downloaded_size:
            raise _error(f"uploaded asset size differs from local asset: {name}")
        if local_digest != downloaded_digest:
            raise _error(f"uploaded asset digest differs from local asset: {name}")
        if item["size"] != downloaded_size:
            raise _error(f"GitHub asset size does not match downloaded bytes: {name}")
        if item.get("digest") is not None and item["digest"] != f"sha256:{downloaded_digest}":
            raise _error(f"GitHub asset digest does not match downloaded bytes: {name}")
        verified.append(
            {
                "id": item["id"],
                "name": name,
                "size": downloaded_size,
                "digest": downloaded_digest,
            }
        )
    return verified


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("local_root", type=Path)
    parser.add_argument("metadata", type=Path)
    parser.add_argument("downloaded_root", type=Path)
    args = parser.parse_args()
    for item in verify_uploaded_assets(args.local_root, args.metadata, args.downloaded_root):
        print(f"verified {item['name']} size={item['size']} sha256={item['digest']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
