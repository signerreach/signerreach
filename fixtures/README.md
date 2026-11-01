# Fixtures

Each directory here is one case: three input documents and the result the tool
must give for them. The test `crates/signerreach-cli/tests/fixtures.rs` runs
every directory through the real `signerreach` binary.

## Layout

| File | Content |
|---|---|
| `snapshot.json` | The account, as `signerreach.snapshot/1` |
| `plan.json` | The proposed transactions, as `signerreach.plan/1` |
| `availability.json` | Which signers are available, as `signerreach.availability/1` |
| `expected.json` | What the tool must produce (below) |
| `after.json` | Optional replay-only follow-up attempts, consumed by `tools/replay` only |

Directory names are the lower-case fixture id followed by a short description,
for example `fx-05b-two-changes-in-two-transactions`.

## `expected.json`

| Field | Meaning |
|---|---|
| `id` | The fixture id, such as `FX-05b`. Unique. |
| `summary` | One sentence on what the case shows. |
| `source` | `chain` or `synthetic` (below). |
| `evidence_cases` | For `chain` only: the case ids in `docs/evidence/testnet-2026-10-13/README.md` that back this fixture. |
| `verdict` | `SAFE WITHIN SCOPE`, `UNSAFE` or `INCONCLUSIVE`. |
| `exit_code` | 0, 1 or 2, matching the verdict. |
| `codes` | The diagnostic codes expected, without repeats. For rejected input these are the codes of the input problems. |
| `transactions` | The result of each transaction in order (`accepted`, `rejected`, `failed_to_apply`, `undecided`, `not_evaluated`). Empty when there are no transactions or the input was rejected. |

## Two kinds of fixture

- **`chain`**: the outcome was observed on a real network. Each one names the
  Testnet cases that back it, and the test checks that those cases appear in the
  evidence table. The observed parts (the `transactions` results and the
  `verdict`) must never be changed to match the tool; if the tool disagrees with
  them, the tool is wrong or the evidence needs repeating. The `codes` are not
  observations: they follow from the rules in `docs/SPEC.md` applied to the
  observed outcome, and are fixed by the specification.
  The chain fixtures can be replayed on a fresh Testnet with
  [`tools/replay`](../tools/replay/README.md).
- **`synthetic`**: a case that tests the tool's own handling of input and
  assumptions. It says nothing about Stellar and must never be presented as
  evidence about the network.

## Replay-only follow-up attempts

`after.json` is optional. It does not form part of the checker's three inputs,
does not change `expected.json`, and is used only by `tools/replay`. Its schema
is `signerreach.replay-after/1`; each ordered attempt has an `id`, an explicit
`signed_by` list of fixture G addresses, `set_options` operations, and an
expected stage. There may be at most 100 attempts and 100 operations per
transaction. `home_domain` is limited to 32 UTF-8 bytes. Expected result-code
names must be known to the pinned Stellar SDK's XDR definitions, and result
code objects accept only `transaction` and ordered `operations` fields. A
listed signer must be declared `available` by the fixture's `availability.json`.
Omitted `after.json` means no follow-up attempts; invalid UTF-8, malformed JSON,
duplicate keys, or unsupported fields fail validation before replay.

The expected stages are `accepted`, `rejected_at_submission`, and
`failed_to_apply`. The latter requires ledger history to confirm inclusion and
a failed transaction result. A submission rejection requires result codes plus
a bounded history lookup that did not find the hash and unchanged signer state
and sequence. Bounded absence is not absolute proof of non-inclusion. Runtime
`unconfirmed` and `not_run` outcomes never count as matches. Result-code arrays
are compared in order. Replay output retains only an allowlisted, bounded
sanitized subset of Horizon responses.

The offline dry run validates the optional file and builds/signs its attempts,
but reports `not_submitted` and never compares expected network results.
Live follow-up replay remains Testnet-only and protocol 29-only. It is not run
in PR CI. Historical observation files are immutable; a new replay creates new
results and never edits the October 13 or October 19 records.

## Adding a fixture

1. Create a directory with the four files above.
2. Write `expected.json` by hand from the specification (`docs/SPEC.md`), not by
   copying the tool's output, so the fixture checks the tool rather than echoing it.
3. Run `cargo test -p signerreach-cli --test fixtures`.
