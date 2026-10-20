# Walkthrough: an unsafe change, and a corrected plan

This follows one case from start to finish: a change that looks routine and
succeeds on the network, but leaves the account unable to be administered. It
shows what SignerReach reports, what the network did in the real test behind it,
and a corrected plan.

Run every command from the repository root. Build first with `cargo build`, then
use `target/debug/signerreach` (or `cargo run -p signerreach-cli --`).

## 1. The situation

An account `GAAQC...DZ7H` has two signers, each of weight 1. All three thresholds
are 1, so either key can do anything. The owner wants to require both keys for
sensitive changes, so the plan is one transaction, signed with key A:

- set the high threshold to 2.

The owner also knows that key B (`GABAE...JXA`) is lost. The files are the
fixture [`fx-01-raise-high-with-lost-signer`](../../fixtures/fx-01-raise-high-with-lost-signer):
the account (`snapshot.json`), the plan (`plan.json`), and which keys can be used
(`availability.json`: A available, B lost).

## 2. Check the plan

```bash
target/debug/signerreach check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan fixtures/fx-01-raise-high-with-lost-signer/plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The exit code is 1. The report begins:

```
VERDICT: UNSAFE
Reason: signers declared available weigh 1, below the HIGH threshold 2
```

The trace shows why. The transaction itself is fine: the account's own key meets
the HIGH threshold of 1 that applies before the change, so the network accepts it.

```
  Transaction raise-high
    operation 0: set high threshold to 2
    check envelope [LOW]: needs 1, signatures give 1: met
    check operation 0 [HIGH]: needs 1, signatures give 1: met
    result: accepted
```

The problem is what is left behind:

```
  Baseline declaration: UNSAFE
    LOW    threshold   1  total 2  available 1  unknown 0: reachable
    MEDIUM threshold   1  total 2  available 1  unknown 0: reachable
    HIGH   threshold   2  total 2  available 1  unknown 0: NOT reachable: the signers that could meet it are declared lost

  SR-LOSS-01 [UNSAFE] (baseline) signers declared available weigh 1, below the HIGH threshold 2
```

After the change, only key A is usable and it weighs 1. The HIGH threshold is 2,
and the only other weight belongs to the lost key. Nobody can change the signers
or thresholds again.

## 3. What the network did

This case was run on Stellar Testnet twice, with throwaway keys. The fixture's
`transactions` result is taken from those runs, not from the tool.

| Run | Step | Observed |
|---|---|---|
| [2026-10-13](../evidence/testnet-2026-10-13/README.md), case 1a | the plan, signed by A | accepted, ledger 5180919, `261eedc57d15c5b00c31eb2f94eb6ef1bb86afb1fa78fb668f5787ed28156f00` |
| 2026-10-13, case 1b | afterwards, a change to the HIGH threshold signed by A alone | rejected at submission with `op_bad_auth` |
| 2026-10-13, case 1c | the same change signed by A and B (the control) | accepted, ledger 5180920 |
| [2026-10-19](../evidence/replay-2026-10-19/README.md), FX-01 | the plan, signed by A, on a fresh account | accepted, ledger 5280621, `30c3b9a24bb49029adcb2daad1028b4aff0bb71df8c9964d3976953ff08c408f` |

Case 1b is the lockout itself: the account refuses a change from the only key
that is still available. Case 1c shows that the account was not broken in some
other way, because both keys together could still change it. The 2026-10-19 run
repeated only the plan's own transaction. It did not repeat 1b and 1c.

## 4. A corrected plan

The aim was a HIGH threshold of 2 that the available key can still meet. Raise
key A's own weight to 2 in the same transaction
([`corrected-plan.json`](corrected-plan.json)):

- set the master weight to 2;
- set the high threshold to 2.

```bash
target/debug/signerreach check \
  --snapshot fixtures/fx-01-raise-high-with-lost-signer/snapshot.json \
  --plan docs/walkthrough/corrected-plan.json \
  --availability fixtures/fx-01-raise-high-with-lost-signer/availability.json
```

The exit code is 0:

```
VERDICT: SAFE WITHIN SCOPE
...
    check operation 0 [HIGH]: needs 1, signatures give 1: met
    check operation 1 [HIGH]: needs 1, signatures give 1: met
    result: accepted
...
    HIGH   threshold   2  total 3  available 2  unknown 0: reachable
```

Both operations are authorized against the account as it was before the
transaction, so key A's weight of 1 is enough for both. Afterwards key A weighs 2
and meets the HIGH threshold of 2 on its own.

Sent as two separate transactions, threshold first, the same two changes fail:
the tool reports the second transaction as rejected (`SR-AUTH-01`, it needs HIGH
2 and key A gives 1), because the first has already raised the threshold. That
difference is the reason the tool treats the transaction, not the operation, as
its unit. The network behaviour behind it is shown by the fixtures
[`FX-05a`](../../fixtures/fx-05a-two-changes-in-one-transaction) and
[`FX-05b`](../../fixtures/fx-05b-two-changes-in-two-transactions), which make a
different pair of changes as one transaction and as two.

## 5. What this does and does not show

- **The corrected plan was not run on a network.** The verdict above comes from
  the tool. What backs the mechanism it relies on, two changes in one
  transaction authorized against the earlier state, is the Testnet observation
  behind `FX-05a`, not a run of this exact plan.
- **The result depends on what you declare.** The tool cannot know that key B is
  lost, or that key A is safe. If key A is also lost, no plan helps.
- **It applies to the ledger the snapshot was read at.** Here that is 5180915, a
  Testnet ledger from 2026-10-13. A live account has to be read again, with
  `signerreach fetch`, just before acting.
- **The coverage list at the end of every report states what was not modelled.**
