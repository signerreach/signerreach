# Publishing v0.1.1

The `Publish v0.1.1` workflow is intentionally manual. It has no `push` or
`pull_request` trigger and does not publish from a branch update.

Before any publication, an authorized maintainer must confirm that the
protected environment used by the workflow is configured (it was created for
v0.1.0), and configure it if it is missing:

1. Open the repository's **Settings → Environments**.
2. Create an environment named exactly `signerreach-release`.
3. Add the maintainer or release team as **Required reviewers**.
4. Enable **Prevent self-review** when that option is available.
5. Restrict deployment branches or tags to `main` if the repository policy
   exposes that environment control.

The workflow's publish job has `contents: write` only after this environment
gate. The verification and native build jobs have read-only permissions. The
`PUBLISH v0.1.1` input records operator intent only; it is not an approval and
is never equivalent to a protected-environment review. GitHub does not expose a
reliable read-only API signal for all of the environment's required-reviewer
settings to this workflow, so maintainer verification of the configuration is
mandatory before each use and after any environment-policy change. Do not
run the publication workflow if that protected environment is not configured.

## Manual publication

1. Confirm that the current `main` SHA is the intended reviewed commit and
   that the existing CI and documentation checks are green.
2. Open **Actions → Publish v0.1.1 → Run workflow**.
3. Select the `main` branch.
4. Enter the full 40-character `main` SHA in **source_commit**.
5. Select **PUBLISH v0.1.1** in **confirmation**, then click **Run workflow**.
6. When the `Approve, tag and publish release` job pauses, choose **Review
   deployments**, select `signerreach-release`, and click **Approve and deploy**.

The workflow then rechecks that `main` has not moved, the version is `0.1.1`,
the tag and release do not already exist, and the required checks are green. It
builds Linux, macOS and Windows packages from that exact commit, verifies their
manifests, checksums and contents, renders final release notes, creates an
annotated `v0.1.1` tag, creates a draft release with nine verified assets, and
publishes it only after the asset set is complete.

If a pre-publication step fails after this workflow creates a tag or draft
release, cleanup removes only a release whose ID was returned by this run and
whose current API state is still a draft for the expected tag, plus the tag if
its annotated object is confirmed to belong to this run and no release lookup
is public or unresolved. It never overwrites an existing tag or release. A
published release, a foreign resource, or any ambiguous API response is
preserved for maintainer investigation.

The published notes state the Protocol 29 and 30 scope and its evidence. They do
not claim crates.io publication or an independent security audit.

## Preparing a later release

The workflow is written for one version at a time, on purpose. For the next
release, change these together in one reviewed pull request: `version` in
`Cargo.toml` and the two path dependencies, `Cargo.lock` (`cargo update
--workspace`), `VERSION` in `release-prep.yml`, and in `release-publish.yml` the
workflow name, the `PUBLISH` option and its check, `VERSION`, `TAG`, the
concurrency group, the version check on `Cargo.toml` and the notes template
path. Add the matching `.github/release-notes/vX.Y.Z.md.tmpl`.
