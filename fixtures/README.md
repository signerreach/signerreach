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

## Adding a fixture

1. Create a directory with the four files above.
2. Write `expected.json` by hand from the specification (`docs/SPEC.md`), not by
   copying the tool's output, so the fixture checks the tool rather than echoing it.
3. Run `cargo test -p signerreach-cli --test fixtures`.
