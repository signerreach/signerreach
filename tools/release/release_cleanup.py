#!/usr/bin/env python3
"""Fail-closed cleanup decisions for the publication workflow."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def cleanup_plan(state: dict[str, Any]) -> dict[str, Any]:
    """Return deletion permissions for one observed cleanup state.

    The caller must have obtained the observations from the GitHub API. An
    unknown observation never authorizes deletion. A release can be deleted
    only when its ID was returned by this run's create request and the current
    API response confirms that it is still a draft for the expected tag.
    """

    tag = state.get("tag", {})
    release = state.get("release", {})
    owned_tag_object_sha = state.get("owned_tag_object_sha")
    owned_release_id = state.get("owned_release_id")

    tag_owned = (
        tag.get("state") == "found"
        and bool(owned_tag_object_sha)
        and tag.get("object_sha") == owned_tag_object_sha
        and tag.get("object_type") == "tag"
    )
    release_is_owned_draft = (
        release.get("state") == "draft"
        and bool(owned_release_id)
        and release.get("id") == owned_release_id
        and release.get("tag_name") == state.get("tag_name")
    )
    release_is_confirmed_absent = release.get("state") == "absent"

    delete_release = release_is_owned_draft
    delete_tag = tag_owned and (release_is_owned_draft or release_is_confirmed_absent)

    if delete_release and delete_tag:
        reason = "delete the workflow-owned draft and its workflow-owned tag"
    elif delete_tag:
        reason = "delete the workflow-owned tag after confirmed release absence"
    elif release.get("state") in {"published", "unknown", "foreign"}:
        reason = "preserve resources because a release is public, foreign, or unresolved"
    elif not tag_owned:
        reason = "preserve the tag because ownership is absent or unresolved"
    else:
        reason = "preserve resources because cleanup ownership is not confirmed"

    return {
        "delete_release": delete_release,
        "delete_tag": delete_tag,
        "release_id": owned_release_id if delete_release else None,
        "reason": reason,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("state", type=Path)
    args = parser.parse_args()
    state = json.loads(args.state.read_text(encoding="utf-8"))
    print(json.dumps(cleanup_plan(state), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
