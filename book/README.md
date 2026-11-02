# SignerReach documentation book

The mdBook source is in [`src/`](src/SUMMARY.md). The book is maintained alongside, but does not replace, the authoritative [specification](../docs/SPEC.md), [architecture](../docs/architecture.md), [security review](../docs/security-review.md), [evidence](../evidence/index.md), and [contributing guide](../CONTRIBUTING.md).

Build locally with mdBook 0.5.4:

```sh
mdbook build book
python3 tools/docs/check_book.py book
```

Run the source-only link check even without mdBook:

```sh
python3 tools/docs/check_book.py book --source-only
```

The book is designed for GitHub Pages but is not considered published until the `Docs deploy` job has passed and the public URL is verified. Details of enabling deployment belong to the repository maintainer.
