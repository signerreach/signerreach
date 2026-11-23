# Reserve, fee and operation-order boundaries on Testnet, 2026-11-23

Seven cases run on Stellar Testnet at protocol 30, each on a fresh account whose
balance was set to an exact number of stroops. For each, `signerreach check` was
asked about the plan first, exactly as a user would ask (`fetch`, then `check`),
and then the plan was submitted. The checker's answer and the network's answer
agreed in all seven. This is the record behind rules R10 to R14 in
[`docs/SPEC.md`](../../SPEC.md), for the cases listed below and no others.

[`../testnet-2026-11-20-reserve-gap`](../testnet-2026-11-20-reserve-gap/README.md)
is the earlier record of the failure that led to those rules. It stays unchanged.

## What was run

- Date and time: 2026-11-23T15:56:02.308Z to 2026-11-23T15:58:13.130Z.
- Network: Stellar Testnet, protocol **30** at the start and
  at the end of the run, ledgers 5885143 to 5885170.
  Horizon `30.0.1-6f7cff3903be1df4fb980a12910a1313c5d7d8dd`, core `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)`.
  Base reserve 5,000,000 stroops and base fee
  100 stroops, read from the ledger record at the start.
- Checker: `signerreach 0.1.1`, built from `main` at commit
  `70aaaba5a4c906d1bb20f03fd5235581af423257` (the merge of pull request #53) with
  `cargo build --release --locked -p signerreach-cli` and `rustc 1.98.0`. Its
  SHA-256 is `e5f44450ce034738b0f820d9e86f72972252fe83219838ab3d3b2ac458df3e54`. The working tree was clean.
- Script: [`tools/replay/reserve-boundaries.mjs`](../../../tools/replay/reserve-boundaries.mjs),
  with `@stellar/stellar-sdk` from `tools/replay/package.json`, Node v24.16.0.
  Keys are generated in memory for each case and are not printed or saved. Only
  public addresses appear in these files.
- The expected result of each case is written in the script from the table in
  section 10 of the specification. It is not read from the checker's output.

## How a case works

1. A funder account (Friendbot) creates the account under test with a starting
   balance of the target plus the fee of its setup transaction.
2. A setup transaction adds the signers the case needs, so the account has the
   stated number of sub-entries. Thresholds stay 0, so the account's own key
   authorizes every plan.
3. The account is read back. If its balance is not exactly the target, or its
   sub-entry count is wrong, the case is recorded as unusable and does not count.
   None was unusable.
4. `signerreach fetch` reads the account, and `signerreach check` is run on the
   plan with every signer declared available.
5. The plan is submitted one transaction at a time, each signed by the account.
6. For a transaction the network refuses, Horizon is asked whether it is in a
   ledger. One that is in a ledger and failed (`tx_failed`) has a record with
   `fee_charged`. One that was rejected at submission is looked for four times,
   two seconds apart, and not found. Not finding it is cautious evidence, not proof
   that it can never appear.

## What was observed

The checker answers once for the whole plan, so its answer is on the first row of
each case, and "same" on a later row of that case.

| Case | Balance (stroops) | Sub-entries | Transaction | Network | Ledger | `fee_charged` | Result codes | Checker's answer |
|---|---|---|---|---|---|---|---|---|
| FX-10a | 20,000,100 | 1 | add-d | accepted | 5885149 | none | none | SAFE WITHIN SCOPE |
| FX-10b | 20,000,099 | 1 | add-d | in a ledger, failed | 5885152 | 100 | `tx_failed` / `op_low_reserve` | UNSAFE `SR-RESERVE-01` |
| FX-11a | 20,000,200 | 2 | swap | accepted | 5885155 | none | none | SAFE WITHIN SCOPE |
| FX-11b | 20,000,200 | 2 | swap | in a ledger, failed | 5885158 | 200 | `tx_failed` / `op_low_reserve`, `op_success` | UNSAFE `SR-RESERVE-01` |
| FX-12 | 15,000,100 | 1 | reweight | accepted | 5885161 | none | none | SAFE WITHIN SCOPE |
| FX-14 | 15,000,099 | 1 | reweight | rejected at submission | none | none | `tx_insufficient_balance` | UNSAFE `SR-RESERVE-01` |
| FX-17 | 15,000,150 | 1 | add-d | in a ledger, failed | 5885168 | 100 | `tx_failed` / `op_low_reserve` | UNSAFE `SR-RESERVE-01` |
| FX-17 | 15,000,150 | 1 | reweight | rejected at submission | none | none | `tx_insufficient_balance` | same |

The balance after each case, which shows who paid a fee:

| Case | Before the plan | After the plan |
|---|---|---|
| FX-10a | 20,000,100 | 20,000,000 |
| FX-10b | 20,000,099 | 19,999,999 |
| FX-11a | 20,000,200 | 20,000,000 |
| FX-11b | 20,000,200 | 20,000,000 |
| FX-12 | 15,000,100 | 15,000,000 |
| FX-14 | 15,000,099 | 15,000,099 |
| FX-17 | 15,000,150 | 15,000,050 |

What each case shows, and nothing beyond it:

- **R11, the boundary.** With one sub-entry, an account holding exactly 20,000,100
  stroops (minimum balance 15,000,000 + base reserve 5,000,000 + fee 100) adds a
  signer (FX-10a). With one stroop less it does not: `op_low_reserve` (FX-10b).
- **R12, the fee is kept.** All three failed transactions that were included in a
  ledger were charged their full fee (100, 200 for two operations, and 100), and
  the balances afterwards are the balances before less that fee. The transactions
  rejected at submission (FX-14, and the second one in FX-17) charged nothing.
- **R13, the fee must be payable.** With 99 stroops available and a fee of 100, the
  network rejected the transaction with `tx_insufficient_balance` before applying
  it (FX-14). In FX-17 the first transaction failed with `op_low_reserve` and kept
  its 100, which left 50 available, so the second was rejected the same way.
- **R14, order inside a transaction.** At the same balance, `[remove C, add D]` was
  accepted (FX-11a) and `[add D, remove C]` failed with `op_low_reserve` (FX-11b).
  In the failed one Horizon lists the second operation as `op_success`, but the
  transaction is atomic: the sub-entry count afterwards is still 2, so the removal
  did not take effect, and only the fee was paid.
- **R10, re-weighting.** With 100 stroops available, enough for the fee and nothing
  more, changing the weight of an existing signer was accepted (FX-12).
- **The checker agrees with the network** in every case, and each snapshot that
  `fetch` wrote matches the account it was read from (balance, sub-entry count and
  base reserve).

## What this does not show

These stay from the stellar-core source reading only, and are not recorded on a
network by this run: selling liabilities (FX-13), the 1000 sub-entry limit
(FX-16), sponsorship, the order in which the signer limit, the sub-entry limit and
the reserve are checked, and any account whose base reserve is not 5,000,000
stroops. The run is one pass on one day. It does not show that a plan which the
checker calls safe will succeed in general, only that these seven agreed.

## Transaction hashes

| Case | Transaction | Hash |
|---|---|---|
| FX-10a | add-d | `b5da0a6a48b6eedb30615c5969bc7f539d247fabb7e3ec81d5a32bd3b9107eb4` |
| FX-10b | add-d | `27083aa6308477e3b7c7681cc68434899023e454e3453b0ff503cdf13d3df4b1` |
| FX-11a | swap | `5e06e00efe50b4842bc438f84b3b1bebca485621dc48974a09f0ea2822e6dbf2` |
| FX-11b | swap | `cea8b8d30ac5d93056ebc6bdaef21bde9fac64c20e05efe31b4e9e9c42babaa2` |
| FX-12 | reweight | `f932924e6b77cd6c33b4a01f3479fd50b9fa6ab4866131b17229e5167e6c0b5d` |
| FX-14 | reweight | `a1a9abe86c3ff4d9b7abcec421ee752ec684cf62e690981b65a03354451a89b5` |
| FX-17 | add-d | `a502926f77c0fe62154d6df57cc908db37e94080e55d9bb7483574a19fa8a0fa` |
| FX-17 | reweight | `728cf8cfd3c13a8e4d63a1254d126738ca60ba3ccf54f1dda1007025b334f0e5` |

## Files

| File | Content |
|---|---|
| `results.json` | Everything the run recorded, including the run information and each case's checks. |
| `fx-*/snapshot.json`, `plan.json`, `availability.json` | The three inputs `check` was given for that case. |
| `fx-*/check.txt`, `check.json` | The checker's report, in both formats. |

Testnet is reset from time to time, so these records may not be readable from the
network later. The files here are the record.
