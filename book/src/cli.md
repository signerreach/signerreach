# CLI reference

There are two implemented commands: **`check`** and **`fetch`**. The CLI does not hold private keys and never signs or submits transactions.

## `signerreach check`

```sh
signerreach check \
  --snapshot snapshot.json \
  --plan plan.json \
  --availability availability.json
```

The inputs are a validated account snapshot, a sequence of planned transactions and a declaration of signer availability. For structured output, add `--format json`. The supported formats are **text** (default) and **json**. Markdown output and a JSON Schema command are [proposed contributor enhancements](contributing.md), not implemented features.

| Exit | Meaning |
| --- | --- |
| `0` | `SAFE WITHIN SCOPE` |
| `1` | `UNSAFE` |
| `2` | `INCONCLUSIVE`, including rejected input |
| `64` | CLI usage error or unreadable input file |

## `signerreach fetch`

```sh
signerreach fetch --network testnet --account G... > snapshot.json
```

`fetch` performs constrained, read-only requests to Horizon. It can use an explicitly supplied `--horizon-url`. It returns `0` for a snapshot, `64` for incorrect arguments and `69` if the account cannot be read. Never assume a snapshot is fresh just because a request succeeded.

## Schema and diagnostics

The exact input layouts and diagnostic meanings are maintained in the [project specification](https://github.com/signerreach/signerreach/blob/main/docs/SPEC.md). The [fixture guide](https://github.com/signerreach/signerreach/blob/main/fixtures/README.md) explains how the cases are written and what is observed versus synthetic.

Use the [getting-started example](getting-started.md) before incorporating SignerReach into automation.
