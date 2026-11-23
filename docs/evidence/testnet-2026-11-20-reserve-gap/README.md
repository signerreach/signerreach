# A false SAFE result on Testnet at protocol 30, 2026-11-20

One run on Stellar Testnet. The checker reported `SAFE WITHIN SCOPE` for a plan
that adds one signer. The network included the transaction in a ledger and it
failed with `op_low_reserve`. This is the observation that the balance, reserve
and fee rules (R10 to R14 in [`docs/SPEC.md`](../../SPEC.md)) were written to
remove, and the fixture `FX-10c` is built from it.

It records one case. It does not record the boundary cases, the order of
operations, selling liabilities, the 1000 sub-entry limit or sponsorship. The
specification says which rules rest on the stellar-core source alone.

## What was run

- Date and time: 2026-11-20, 13:50:56Z to 13:51:26Z (`run-1/run-log.json`).
- Network: Stellar Testnet, `https://horizon-testnet.stellar.org`, protocol 30
  (`run-1/snapshot.json`), core `stellar-core 30.0.0
  (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)` as Horizon reports it today.
- Checker: `signerreach` version 0.1.1, as `run-1/check.json` states. The binary's
  hash was not written down by the run, so it is not claimed here. It is the
  checker as it was **before** the balance, reserve and fee rules, so it did not
  read the balance at all.
- Script: `reserve-gap.mjs`, as it ran, using `@stellar/stellar-sdk` 17.3.0. It
  creates throwaway keys in memory and prints and stores no secret. Everything it
  wrote is in `run-1/`.

## What happened

1. Friendbot funded a throwaway account. That account created a second account,
   the one under test, with a starting balance of 1.0010000 XLM
   (transaction `48d3220b…`, ledger 5831806).
2. The account under test had balance 10,010,000 stroops, no selling liabilities,
   no sub-entries and no sponsorship. The base reserve in ledger 5831809 is
   5,000,000 stroops (`run-1/horizon-ledger-5831809.json`), so its minimum balance
   was 2 x 5,000,000 = 10,000,000 and 10,000 stroops were available.
3. `signerreach fetch` and `signerreach check` were run as a user would run them,
   on a plan that adds one signer with weight 1, signed by the account itself
   (`run-1/plan.json`). The snapshot is as of ledger 5831808, captured at
   13:51:21Z. The checker answered **`SAFE WITHIN SCOPE`** (`run-1/check.txt`).
4. The same change was submitted to Testnet, signed by the account. The network
   answered HTTP 400 with `tx_failed` and one operation result, `op_low_reserve`
   (`run-1/run-log.json`).

## What the network recorded

Horizon's record of the transaction, read on 2026-11-23
(`run-1/horizon-transaction.json`):

| Field | Value |
|---|---|
| `hash` | `c852ac7a5350158aedb8c94ccc0a1c3f949de1af2d3546555a459e9a77764706` |
| `ledger` | 5831809, closed 2026-11-20T13:51:25Z |
| `successful` | `false` |
| `fee_charged` | `100` stroops |
| `result_xdr` | `AAAAAAAAAGT/////AAAAAQAAAAAAAAAF/////wAAAAA=` (fee 100, `txFAILED`, one `SetOptions` result, `SET_OPTIONS_LOW_RESERVE`) |

Two facts follow, and only these:

- A signer addition that leaves less than one base reserve available fails with
  `op_low_reserve` at protocol 30, although the transaction is authorized and the
  account stays reachable. This is rule R11.
- The failed transaction **is in a ledger and was charged its fee** (`fee_charged`
  100). It is not rejected at submission. This is what R12 says about a
  transaction that fails when applied.

A later read agrees with the fee. On 2026-11-23 `signerreach fetch` read the same
account (ledger 5884851, `run-1/account-after-2026-11-23.snapshot.json`): balance
10,009,900 stroops, which is the 10,010,000 it started with less the 100 of the
failed transaction. No other transaction was sent from it. That `fetch` is the
new one, so the snapshot also has the `reserve` object.

## Two things about the run itself

- The script printed the failed transaction's hash as a list of byte values, not
  as hex (`computed_hash` in `run-1/run-log.json`). The list converts to the hash
  above, and Horizon returns that hash with the ledger and result shown. The
  hash and the ledger were not in the run's own output; they come from the
  Horizon record fetched afterwards.
- The checker's answer in this run is not the checker's answer now. After the
  change that this record motivated, the same account would give `UNSAFE` with
  `SR-RESERVE-01`, which is what the fixture `FX-10c` expects. That claim rests
  on the fixture and the unit tests, not on a new network run.

## Files

| File | Content |
|---|---|
| `reserve-gap.mjs` | The script that ran. |
| `run-1/run-log.json` | Each step with its time, and the network's answer. |
| `run-1/snapshot.json`, `plan.json`, `availability.json` | The three inputs `check` was given. |
| `run-1/check.txt`, `check.json` | The checker's report, in both formats. |
| `run-1/horizon-transaction.json` | Horizon's record of the failed transaction. |
| `run-1/horizon-ledger-5831809.json` | Horizon's record of the ledger it was in. |
| `run-1/account-after-2026-11-23.snapshot.json` | The account read again three days later, by the new `fetch`. |

Testnet is reset from time to time, so these records may not be readable from the
network later. The files here are the record.
