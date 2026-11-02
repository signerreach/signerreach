# Publishing v0.1.0

The `Publish v0.1.0` workflow is intentionally manual. It has no `push` or
`pull_request` trigger and does not publish from a branch update.

Before the first publication, an authorized maintainer must configure the
protected environment used by the workflow:

1. Open the repository's **Settings → Environments**.
2. Create an environment named exactly `signerreach-release`.
3. Add the maintainer or release team as **Required reviewers**.
4. Enable **Prevent self-review** when that option is available.
5. Restrict deployment branches or tags to `main` if the repository policy
   exposes that environment control.

The workflow's publish job has `contents: write` only after this environment
gate. The verification and native build jobs have read-only permissions. If
the environment has no required reviewer, the workflow's confirmation input is
not a substitute for the required protection configuration.

## Manual publication

1. Confirm that the current `main` SHA is the intended reviewed commit and
   that the existing CI and documentation checks are green.
2. Open **Actions → Publish v0.1.0 → Run workflow**.
3. Select the `main` branch.
4. Enter the full 40-character `main` SHA in **source_commit**.
5. Select **PUBLISH v0.1.0** in **confirmation**, then click **Run workflow**.
6. When the `Approve, tag and publish release` job pauses, choose **Review
   deployments**, select `signerreach-release`, and click **Approve and deploy**.

The workflow then rechecks that `main` has not moved, the version is `0.1.0`,
the tag and release do not already exist, and the required checks are green. It
builds Linux, macOS and Windows packages from that exact commit, verifies their
manifests, checksums and contents, renders final release notes, creates an
annotated `v0.1.0` tag, creates a draft release with nine verified assets, and
publishes it only after the asset set is complete.

If a pre-publication step fails after this workflow creates a tag or draft
release, the same job removes the tag and draft release that it created. It
never overwrites an existing tag or release. A published release is preserved
for maintainer investigation if the final API response is ambiguous.

The published notes retain the Protocol 29-only boundary. They do not claim
Protocol 30 compatibility, crates.io publication, live Testnet submission or
an independent security audit.
