# SignerReach V1 Specification (draft 0.1)

Date: 2026-10-14
Current publication status (2026-11-27): SignerReach v0.1.2 was published from verified main commit [`9d78409e5d49ca8cd7c974728ff1f8ecb59a5642`](https://github.com/signerreach/signerreach/releases/tag/v0.1.2). v0.1.1 was published on 2026-11-04 from [`f950a821044c479c29550e6e003612b2de7b9175`](https://github.com/signerreach/signerreach/releases/tag/v0.1.1). The first release, v0.1.0, was published on 2026-11-02 from [`e1a37cea9101bbf2cb68372f111c6e4a5ecd1ba4`](https://github.com/signerreach/signerreach/releases/tag/v0.1.0). This file retains the original Draft 0.1 specification and fixture IDs. The Rust core, read-only Horizon adapter, `check` and `fetch` commands, text and JSON reports, and development Testnet replay harness are implemented and tested. Markdown reports and `signerreach schema KIND` remain planned in issues [#2](https://github.com/signerreach/signerreach/issues/2) and [#1](https://github.com/signerreach/signerreach/issues/1). The project has not been independently audited. v0.1.0 accepts protocol 29 only. Since v0.1.1 the checker also accepts protocol 30, after the chain fixtures and the FX-01 follow-up matched on a Testnet running protocol 30 ([evidence](evidence/replay-2026-11-04-protocol-30/README.md)). Sections below describe intended V1 behavior where they differ from the current implementation; the status notes identify those differences. The balance, reserve and fee rules (R10 to R14, `reserve` in the snapshot, `fee` in the plan, `SR-RESERVE-01`) are published in v0.1.2; v0.1.1 and v0.1.0 do not have them.

---

## 1. Purpose in one paragraph

SignerReach is an offline command-line checker. Before a person submits a change to the signers or thresholds of a Stellar classic account, it takes a snapshot of the account, an ordered plan of changes, and a declaration of which signers are available, and it answers one question: **after this plan, can the account still be administered, under the stated assumptions?** It answers SAFE WITHIN SCOPE, UNSAFE or INCONCLUSIVE and shows the authorization trace that led to the answer. The shipped binary never holds private signing keys, signs, or submits anything. The separate development replay harness generates temporary Testnet keys and submits fixture transactions.

## 2. What is new compared with what already exists

Stellar Stratum (`tupui/stellar-stratum`) already blocks a final configuration whose thresholds exceed the total signer weight. V1 does **not** present that check as its headline. V1's distinct parts are:

| Part | In Stellar Stratum (as read 2026-10-13) | In SignerReach V1 |
|---|---|---|
| Final-state threshold versus total weight, per class | Yes | Yes (needed as a component) |
| Availability scenarios ("signer X is lost") | Not found | **Yes** |
| Plan as an ordered list of transactions, each authorized against the state left by the previous one | Not found | **Yes** |
| One transaction versus several transactions, modelled separately | Not found | **Yes** |
| Rejection of signatures that would make a transaction fail as "extra" | Not found | **Yes** |
| Offline use from a file, usable in scripts and CI | Browser dApp | **Yes** |
| Reproducible trace and stable machine-readable output | Error text | **Yes** |

If any of these cannot be delivered correctly, V1 is reduced, not padded.

## 3. Scope

### 3.1 In scope

- **Account kind:** one classic account (G address) per run.
- **Network and rules:** Stellar network at **protocol 29 or 30**. The rules were verified on protocol 29, and the chain fixtures were replayed on a Testnet running protocol 30 with the same outcomes. Any other protocol version gives `INCONCLUSIVE` (`SR-RULES-01`).
- **Signer types:** `ed25519` keys, including the account's own master key.
- **Operations in a plan:** `SetOptions` only. A `SetOptions` is *administrative* when it sets master weight, a threshold, or a signer. A home-domain-only or empty `SetOptions` needs MEDIUM authorization. Flag changes cannot be decided from this snapshot format.
- **Transaction source:** the analysed account itself.
- **Availability:** each signer is `available`, `lost` or `unknown`. Default is `unknown`.
- **Balance, reserve and fee, for adding a signer:** when the snapshot carries `reserve` data (5.1), the checker applies R10 to R14: it checks that each transaction's fee can be paid and that each new signer can be afforded, in the order the network applies them. A plan that adds a signer to a snapshot without `reserve` data cannot be `SAFE WITHIN SCOPE` (`SR-STATE-01`).
- **Outputs:** text and JSON reports are implemented. Markdown reporting is planned in issue #2.

### 3.2 Out of scope

Soroban or contract accounts (`C...`), muxed accounts, fee-bump transactions, transaction preconditions beyond what is listed, operation or transaction source accounts other than the analysed account, account flag changes, sponsorship (operations inside a sponsorship sandwich, and sponsored signers), reserve effects of anything other than adding or removing a signer, the network's minimum fee as it may change by validator vote (100 stroops per operation is assumed, 5.2), `pre_auth_tx`, `hash_x` and `ed25519_signed_payload` signers, any operation type other than `SetOptions` (including `AccountMerge`, which needs HIGH but ends the account and is not modelled as a plan step), any protocol other than 29 and 30. Recognized unsupported inputs give `INCONCLUSIVE`. The checker cannot detect every external state constraint from this snapshot format, so those conditions remain outside a safe verdict's scope.

### 3.3 Non-goals

Key collection, signing, broadcasting, custody, fee optimization, payments, asset issuance, a frontend, a backend service, a database, a dashboard, a token, a smart contract, predicting human behaviour.

## 4. Verified rules this specification relies on

Rules R1 to R9 were read in stellar-core source (commit `66defff`) and observed on Stellar Testnet at protocol 29. Rules R10 to R14 have their own source note after the table. The evidence is in `docs/evidence/testnet-2026-10-13/`. The chain-backed cases were replayed at protocol 30 on 2026-11-04 (`docs/evidence/replay-2026-11-04-protocol-30/`); in stellar-core 29.0.0 and 30.0.0 the files `SignatureChecker`, `SignatureUtils` and `SetOptionsOpFrame` are identical. Rule identifiers are used in test fixtures.

| ID | Rule |
|---|---|
| R1 | An account has a master weight (0 to 255), up to 20 additional signers (weights 1 to 255), and low, medium, high thresholds (0 to 255). A signer weight of 0 removes the signer. |
| R2 | The master key can sign only if its weight is above 0. A signature is always needed, even for threshold 0. |
| R3 | A threshold is met when the summed weights of distinct matching signers are greater than or equal to it. A single signer's weight counts as at most 255. |
| R4 | A `SetOptions` that sets master weight, any threshold, or a signer needs the **HIGH** threshold. A `SetOptions` that sets none of those needs MEDIUM. `AccountMerge` needs HIGH. |
| R5 | The transaction envelope itself needs the source account's **LOW** threshold. |
| R6 | All operations of a transaction are authorized against the account state **as it was before that transaction**. Earlier operations in the same transaction do not change who may authorize later ones. |
| R7 | Every signature on a transaction must be used by at least one check, or the transaction is rejected `tx_bad_auth_extra`. Use is decided in signature order, so a signature that is not necessary may or may not be rejected depending on order. |
| R8 | A transaction with insufficient authorization is rejected at submission and never enters a ledger. No fee or sequence number is consumed. |
| R9 | A sequence of separate transactions is evaluated one after another, each against the state left by the previous one. |
| R10 | A `SetOptions` that names a signer key that is **already** a signer changes only that signer's weight. It needs no reserve. Weight 0 removes the signer and lowers the account's sub-entry count by 1. Weight 0 for a key that is not a signer changes nothing. |
| R11 | A `SetOptions` that adds a **new** signer (weight above 0, key not yet a signer) to an account that is not inside a sponsorship sandwich is checked in this order. (a) The account already has 20 additional signers: `op_too_many_signers`. (b) `numSubEntries` is already 1000 or more: `opTOO_MANY_SUBENTRIES`. (c) `available < baseReserve`: `op_low_reserve`. Here `available = balance - minBalance - sellingLiabilities` and `minBalance = (2 + numSubEntries + numSponsoring - numSponsored) * baseReserve`. Equal is allowed. A signer that is added raises `numSubEntries` by 1. |
| R12 | A transaction that is rejected by R8 or R13 consumes nothing. A transaction that passes them and then fails in an operation (R11) is included in a ledger and keeps its fee (R13), and none of its operations take effect. |
| R13 | Before the transaction is applied the network checks that `available >= fee` for the source account, and rejects it with `tx_insufficient_balance` if not (nothing is charged, as in R8). If it passes, the fee is charged first, and the operations then see the balance after the fee. |
| R14 | Operations are applied in order against the state the earlier operations left. This is true only of application; authorization still uses the state before the transaction (R6). So `[remove signer A, add signer B]` can succeed where `[add signer B, remove signer A]` fails `op_low_reserve`. |

**Where R10 to R14 come from.** They were read in stellar-core `v30.0.0`, commit `0e49751aa9ddf892302cca02cea7f3f5d97df9f1`, the version Mainnet and Testnet report: `SetOptionsOpFrame.cpp` (`addOrChangeSigner`, `deleteSigner`, and the order in `doApply`), `SponsorshipUtils.cpp` (`createSignerWithPossibleSponsorship`, `canCreateSignerWithoutSponsorship`, `tooManyNumSubEntries`), `TransactionUtils.cpp` (`getMinBalance`, `getAvailableBalance`) and `TransactionFrame.cpp` (`commonValid`, where `txINSUFFICIENT_BALANCE` is raised after the signature checks). Liabilities count from protocol 10 and sponsorship from protocol 14, so both apply at 29 and 30. One failure was also observed on Testnet at protocol 30: an account with 1.0010 XLM and minimum 1.0 XLM, adding one signer, `tx_failed` / `op_low_reserve`, ledger 5831809 ([evidence](evidence/testnet-2026-11-20-reserve-gap/README.md)). That transaction was included in the ledger and charged its fee (`fee_charged` 100), which is the one observation behind R12. On 2026-11-23 seven cases were run on Testnet at protocol 30, each at an exact balance, and the checker agreed with the network in all seven ([evidence](evidence/testnet-2026-11-23-reserve-boundaries/README.md)). That record covers: R11 at the boundary (exactly enough, and one stroop less), R10 (re-weighting with only the fee available), R12 (three failed transactions kept their fees; two rejected ones charged nothing), R13 (`tx_insufficient_balance` at 99 available against a fee of 100) and R14 (the same balance accepts `[remove, add]` and fails `[add, remove]`). **Observed on Testnet since:** selling liabilities, the 1000 sub-entry limit, sponsorship of one data entry in each direction, and that the signer limit and the sub-entry limit are each checked before the reserve. They were seen in one or two cases each, on 2026-11-27 and 2026-11-28 (see "Known differences from the network" below). **Still from the source reading only:** which of the signer limit and the sub-entry limit is checked first, sponsorship of more than one entry or of entries other than data entries, and any base reserve other than 5,000,000 stroops. Every reserve and fee fixture stays `synthetic` in the corpus (section 10), because a `chain` fixture is one backed by a case in the 2026-10-13 record.

**Known differences from the network.** Two runs on Testnet at protocol 30 on 2026-11-27, of the TypeScript library [`signerreach/reserve`](https://github.com/signerreach/reserve) (31 cases each: [run 1](https://github.com/signerreach/reserve/blob/main/docs/evidence/testnet-2026-11-27/README.md), [run 2](https://github.com/signerreach/reserve/blob/main/docs/evidence/testnet-2026-11-27-run-2/README.md)), found two places where the model above differs from what the network did. The library was built to agree with this checker, and does on 621 generated cases that include both situations, so the checker is expected to have the same two differences. The checker itself was not run against the network on these cases. Neither difference is fixed.

1. *The fee charged.* The checker takes an explicit `fee` to be the amount charged (section 5.2). The network does check `available >= fee` against the bid, as R13 says. But in the three cases where the bid was higher than the minimum and the account could afford it (bids of 101 and 1000 stroops), it charged 100 stroops for each operation, not the bid, and the balance afterwards was 1 to 900 stroops higher than the model gives. This can only make the checker more cautious, as section 5.2 says: a later transaction in the same plan sees less balance than the account has. It was seen with no surge pricing; under surge the charge could be higher, which was not tested. A plan with no explicit `fee` is not affected.
2. *The account's own key.* Section 6.2 says that a transaction that names the account's own key as a signer, to add it or to remove it, passes validation, fails when applied and keeps its fee (`SR-APPLY-01`). On Testnet the network refused both at submission, with `tx_failed` and `op_bad_signer`. There was no ledger record after twelve lookups and the balance did not move, so no fee was charged. The verdict, UNSAFE, is the same. What differs is the account afterwards, and what a later transaction in the plan sees: the model deducts a fee that the network did not. This is the only `SR-APPLY-01` case that was observed this way. The 20-signer limit was seen failing in a ledger with its fee charged (`op_too_many_signers`), and the 1000 sub-entry limit was seen failing in a ledger with its fee charged (`op_too_many_subentries`) in the tier 3 run of 2026-11-28.

The same runs also observed that the signer limit is checked before the reserve (one case: 20 additional signers and less than a reserve available gave `op_too_many_signers`).

**Tier 3 observations, 2026-11-28.** Ten further cases of the TypeScript library on Testnet at protocol 30 ([first pass](https://github.com/signerreach/reserve/blob/main/docs/evidence/testnet-2026-11-28-tier3/README.md), and a [re-run](https://github.com/signerreach/reserve/blob/main/docs/evidence/testnet-2026-11-28-tier3-rerun/README.md) of the one case that the first pass lost to a fault in the tool), all matching the model above. With 1000 sub-entries, adding a signer failed with `op_too_many_subentries` both with plenty of balance and with less than one reserve available, so the 1000 sub-entry limit is checked before the reserve. With 999 sub-entries the 1000th was accepted at exactly one reserve plus the fee above the minimum, and failed one stroop short. With selling liabilities of 1,000,000 stroops, the available balance was reduced by that amount. An account that sponsors one data entry needed one reserve more, and the account whose entry is sponsored one reserve less, to add a signer. These are one or two cases each, at one network state. They do not show which of the signer limit and the sub-entry limit is checked first, or sponsorship beyond one data entry.

## 5. Inputs

All input is untrusted. Files are size-limited (default 1 MiB) and must be UTF-8 JSON. Unknown fields are an error, not ignored.

### 5.1 Account snapshot

```json
{
  "schema": "signerreach.snapshot/1",
  "network": "testnet",
  "account": "G...",
  "provenance": {
    "as_of_ledger": 5180915,
    "captured_at": "2026-10-14T09:00:00Z",
    "source": "horizon",
    "protocol_version": 29
  },
  "master_weight": 1,
  "thresholds": { "low": 1, "medium": 2, "high": 2 },
  "signers": [
    { "key": "G...", "type": "ed25519", "weight": 1 }
  ],
  "sponsored_signers": false,
  "reserve": {
    "balance": 20000100,
    "selling_liabilities": 0,
    "subentry_count": 1,
    "num_sponsoring": 0,
    "num_sponsored": 0,
    "base_reserve": 5000000
  }
}
```

- `provenance.as_of_ledger` and `network` are **required**. Without them the verdict is INCONCLUSIVE (`SR-FRESH-01`).
- The master key is represented only by `master_weight`, not as a signer entry. A snapshot adapter that reads Horizon must remove the master key from the signer list it returns.
- `sponsored_signers: true` makes the run INCONCLUSIVE (`SR-STATE-01`).
- `reserve` is **optional**, and it is an addition to `signerreach.snapshot/1`: a document without it is still valid and gives the same result as before for every plan that adds no signer. When present, all six fields are required and are whole numbers. Amounts are in stroops (1 XLM = 10,000,000 stroops), so no decimal is ever read from text. `balance` is the native balance, `selling_liabilities` the native selling liabilities, `subentry_count`, `num_sponsoring` and `num_sponsored` the account's counts, and `base_reserve` the network's base reserve at `as_of_ledger`. It is refused (`SR-INPUT-01`) if `subentry_count` is above 1000 or below the number of additional signers, if `base_reserve` is 0, or if `2 + subentry_count + num_sponsoring - num_sponsored` is negative. A plan that adds a signer to a snapshot without `reserve` is not refused: it is evaluated as if the signer could be afforded, and the run cannot be `SAFE WITHIN SCOPE` (`SR-STATE-01`, R11).

### 5.2 Plan

```json
{
  "schema": "signerreach.plan/1",
  "transactions": [
    {
      "id": "tx1",
      "fee": 200,
      "signed_by": ["G...", "G..."],
      "operations": [
        { "op": "set_options", "signer": { "key": "G...", "weight": 1 } },
        { "op": "set_options", "signer": { "key": "G...", "weight": 0 } }
      ]
    }
  ]
}
```

- `transactions` is ordered. Transaction boundaries are preserved exactly as written. A plan that is "the same changes" in one transaction and in two transactions is two different plans.
- `signed_by` lists the keys whose signatures the submitted transaction is assumed to contain. It is required because authorization depends on it. The checker does not obtain or verify signatures, and does not infer that a signer marked `lost` cannot have signed a pre-signed transaction. The user must confirm the signatures can actually be supplied.
- A `set_options` operation may carry any of: `master_weight`, `low`, `medium`, `high`, `signer`, `home_domain`, `set_flags`, `clear_flags`. Only `master_weight`, the three thresholds and `signer` change the model. A flag change is parsed but makes the transaction undecidable with `SR-STATE-01`, because the snapshot omits existing account flags. A valid home domain does not change the modeled signer state.
- `transactions` may be empty. An empty plan is a state-only check of the snapshot.
- Any other operation type, including `account_merge`, is rejected with `SR-OP-01`.
- `id` is 1 to 64 characters from letters, digits, `.`, `_` and `-`, and unique within the plan.
- `signed_by` holds at most 20 distinct `G...` keys (a Stellar limit). A key of another kind is `SR-SIGNER-01`.
- A transaction holds 1 to 100 operations (a Stellar limit) and a plan at most 1000 transactions (a limit of this tool).
- A transaction or an operation may carry `source`, a `G...` address. It is read and kept; a source other than the analysed account is reported as `SR-TX-04` when the plan is evaluated against a snapshot.
- A transaction that carries `fee_bump` or `preconditions` is `SR-TX-04`; their content is not read.
- `fee` is optional: the whole fee of the transaction in stroops, a whole number from 1 to 4294967295. It is used only when the snapshot carries `reserve` data (R13). When it is absent the checker assumes **100 stroops for each operation**, which is the base fee both networks reported on 2026-11-23, and says so in the report. A `fee` below 100 stroops for each operation is `SR-STATE-01`: the network's minimum fee is a setting that can change, so the checker does not claim to know that such a transaction is accepted. The checker takes `fee` to be the amount charged. A bid above the amount the network charges can only make the checker more cautious, and that is what was observed: on Testnet on 2026-11-27 the network charged 100 stroops for each operation whatever the bid (see "Known differences from the network" in section 4).
- `home_domain` is at most 32 printable ASCII characters. `set_flags` and `clear_flags` use only the four account flag bits (0 to 15) and may not share a bit.

### 5.3 Availability

```json
{
  "schema": "signerreach.availability/1",
  "default": "unknown",
  "signers": { "G...": "available", "G...": "lost" },
  "scenarios": [
    { "name": "A-lost", "overrides": { "G...": "lost" } }
  ],
  "require_survive_loss_of": 0,
  "intent": "recoverable"
}
```

- `signers` maps keys (including the account key) to `available`, `lost` or `unknown`. Any key not listed takes `default`.
- Each entry of `scenarios` overlays `overrides` on `signers`. The unnamed baseline scenario is always evaluated.
- `require_survive_loss_of` is 0 or 1. When 1, the tool additionally checks that the HIGH threshold stays reachable after the loss of any single available signer.
- `intent` is `recoverable` (default) or `irreversible`. `irreversible` changes presentation only (section 8.4). It never changes the verdict.
- Keys are `G...` addresses; a key of another kind is `SR-SIGNER-01`. A key listed twice is refused.
- A scenario `name` is 1 to 64 characters from letters, digits, `.`, `_` and `-`, and unique. A document holds at most 100 scenarios (a limit of this tool).
- Only `schema` is required. Absent fields default to: `default` unknown, no signers, no scenarios, `require_survive_loss_of` 0, `intent` recoverable.
- A status is resolved in this order: the scenario's override, then the entry in `signers`, then `default`.

## 6. Evaluation

The evaluation is deterministic and has no I/O. Given the same input and rule set it always produces the same output.

### 6.1 Step 1: validate

Reject (INCONCLUSIVE) on: unknown fields; weights or thresholds outside 0 to 255; duplicate signer keys; a signer entry with weight 0 (a weight of 0 removes a signer, so it cannot appear in a state); more than 20 additional signers in any state; more than 20 entries in `signed_by`; a signer key equal to the account; a non-G key; missing provenance; protocol other than 29; any item listed in 3.2. Each rejection carries a diagnostic code. Known fields have a field path. An unknown field is located by its containing object and entry index, so an unvalidated field name is not echoed.

### 6.2 Step 2: authorize each transaction (rules R2 to R7)

For transaction *T* against pre-state *S*:

If any operation changes account flags, the transaction is undecidable (`SR-STATE-01`). Stellar can refuse such a change based on flags absent from the snapshot.

1. **Matching.** Each key in `signed_by` is matched to the account's master key (if master weight above 0) or to an additional signer in *S*. A key that matches nothing is *extra* → `SR-TX-01`, transaction rejected.
2. **Envelope check.** Sum of matched weights (each capped at 255, each key counted once) must be at least `S.low`, and at least one key must match. Otherwise `SR-AUTH-01`.
3. **Operation checks.** For each operation in *T* in order, determine its threshold class with R4 using **S, not intermediate state**. The matched sum must be at least that threshold. Otherwise `SR-AUTH-01` with the failing operation index.
4. **Necessity of signatures (R7).** For each key in `signed_by`, test whether removing it leaves at least one check failing. If every key is necessary, all are used and the transaction is valid. If some key is not necessary, the outcome depends on signature order: the transaction is reported as `SR-TX-02` and the run cannot be SAFE (it becomes INCONCLUSIVE).
5. **Fee (R13), only when the snapshot has `reserve` data.** The transaction's fee is its `fee`, or 100 stroops for each operation if none is given. If `available` is less than the fee, the transaction is rejected, nothing is charged, and the code is `SR-RESERVE-01`.
6. If any step fails, the transaction is **rejected**. The state is unchanged (R8). Evaluation of later transactions continues against the unchanged state so the report can show every failure, but the verdict is already UNSAFE.

If the transaction is valid, charge the fee (when `reserve` data is present) and apply its operations **in order** to produce the next state. Applying an operation that removes a signer that does not exist is a no-op. Setting a signer weight to 0 removes it.

An operation that cannot be applied makes the whole transaction fail when applied. It names the account's own key as a signer (even to remove it), or it would take the account above 20 additional signers, or it would take the account to more than 1000 sub-entries (all `SR-APPLY-01`), or, with `reserve` data, it adds a signer that `available` cannot cover (R11, `SR-RESERVE-01`). In every case the verdict is UNSAFE and the account state is unchanged. The fee is not unchanged: a transaction that fails when applied is in a ledger, so it keeps its fee (R12), and a later transaction in the plan sees the balance without it. The account's own key is the exception: the network refused that at submission and charged nothing (see "Known differences from the network" in section 4). Operations are applied one after another (R14), so at a limit a removal before an addition fits where the reverse order does not.

An operation that adds a signer to a snapshot without `reserve` data is applied as if it succeeded and is reported with `SR-STATE-01` against the transaction and operation. The state after it is what the network would produce if the signer can be afforded, so a loss of access found later in the plan is still reported UNSAFE, and nothing is ever reported SAFE.

### 6.3 Step 3: reachability of the final state

Let *S_n* be the state after the last accepted transaction. For each threshold class (LOW, MEDIUM, HIGH) with threshold *t* and each scenario:

- `W_all` = total capped weight of all signers in *S_n* (master key included if weight above 0), regardless of availability.
- `W_avail` = weight of signers marked `available`.
- `W_unk` = weight of signers marked `unknown`.
- A class is **reachable** in the scenario when `W_avail >= t` and at least one available signer has weight above 0.
- A class is **structurally unreachable** when `W_all < t`, or when no signer has weight above 0.
- A class is **dependent on unknowns** when it is not reachable but `W_avail + W_unk >= t`.

### 6.4 Step 4: verdict per scenario

The verdict is driven by **HIGH**, the administrative threshold. If 6.1 raised any rejection, evaluation stops and the verdict is INCONCLUSIVE; the table below applies only to valid inputs.

| Condition (evaluated in this order) | Verdict | Code |
|---|---|---|
| Any transaction rejected in 6.2 | UNSAFE | `SR-AUTH-01`, `SR-TX-01`, `SR-APPLY-01`, `SR-RESERVE-01` |
| HIGH structurally unreachable | UNSAFE | `SR-LOCK-01` |
| HIGH not reachable and not dependent on unknowns (`W_all >= t` but `W_avail + W_unk < t`) | UNSAFE | `SR-LOSS-01` |
| HIGH dependent on unknowns | INCONCLUSIVE | `SR-AVAIL-01` |
| `require_survive_loss_of = 1` and loss of some available signer makes HIGH unreachable | UNSAFE | `SR-LOSS-01` |
| LOW unreachable while HIGH reachable | INCONCLUSIVE | `SR-TX-03` (administration then needs a different funded transaction source, which V1 does not model) |
| `SR-TX-02` raised in 6.2 | INCONCLUSIVE | `SR-TX-02` |
| HIGH reachable, plan valid, nothing above | SAFE WITHIN SCOPE | none |

**Overall verdict** is the worst across scenarios in the order UNSAFE, INCONCLUSIVE, SAFE WITHIN SCOPE. SAFE WITHIN SCOPE is only ever printed when every declared scenario is SAFE.

MEDIUM unreachable with HIGH reachable is not a failure (HIGH can reset it). It is reported as `SR-LOCK-02` for information.

### 6.5 Why the rules are conservative

A false SAFE can cost someone their account. Recognized unsupported constructs and unverified protocol versions produce INCONCLUSIVE. A result marked SAFE WITHIN SCOPE still depends on the supplied snapshot, real signatures, declared availability, and external conditions that this snapshot cannot establish. The balance is one of them: it is checked for adding a signer and for paying the fee only if the snapshot carries `reserve` data, and it is read as of `as_of_ledger`, so a payment, an offer or a change of the base reserve after that ledger is outside the result.

## 7. Diagnostic codes

The proposal book's code names were explicitly "proposals, not an established API". They are fixed here for V1 with the meanings below. `SR-TX-01` changed meaning from the proposal.

| Code | Meaning | Verdict effect |
|---|---|---|
| `SR-AUTH-01` | A transaction or operation is not authorized by the state before that transaction. Names the earliest failing transaction and operation. | UNSAFE |
| `SR-TX-01` | A signature in `signed_by` matches no signer of the pre-transaction state (would be rejected `tx_bad_auth_extra`). | UNSAFE |
| `SR-TX-02` | A signature is not necessary; whether it is rejected as extra depends on signature order. | INCONCLUSIVE |
| `SR-TX-03` | LOW threshold unreachable; administration requires another funded transaction source. | INCONCLUSIVE |
| `SR-TX-04` | Fee-bump, preconditions, or a source account other than the analysed account. | INCONCLUSIVE |
| `SR-APPLY-01` | The transaction is authorized but would fail when applied: it names the account's own key as a signer, or it would give the account more than 20 additional signers, or more than 1000 sub-entries. (The network refuses the own-key case at submission and charges no fee; see section 4.) | UNSAFE |
| `SR-RESERVE-01` | The balance cannot cover the plan, so the network would reject or fail the transaction: the fee is more than the available balance (`tx_insufficient_balance`, R13), or a new signer leaves less than the base reserve available (`op_low_reserve`, R11). Names the transaction, the operation if there is one, and the shortfall in stroops. | UNSAFE |
| `SR-LOCK-01` | Total weight of all signers is below the HIGH threshold. | UNSAFE |
| `SR-LOSS-01` | HIGH is unreachable with the signers declared available in a scenario. | UNSAFE |
| `SR-LOCK-02` | MEDIUM unreachable (payments blocked); informational. | none |
| `SR-AVAIL-01` | The outcome depends on signers whose availability is `unknown`. | INCONCLUSIVE |
| `SR-SIGNER-01` | A signer of a type other than `ed25519` is present. | INCONCLUSIVE |
| `SR-OP-01` | An operation type outside 3.1. | INCONCLUSIVE |
| `SR-INPUT-01` | Invalid, incomplete or inconsistent input. | INCONCLUSIVE |
| `SR-INPUT-02` | Unsupported account kind (contract, muxed). | INCONCLUSIVE |
| `SR-FRESH-01` | Snapshot provenance missing (no ledger or network). | INCONCLUSIVE |
| `SR-RULES-01` | Protocol or rule version not supported. | INCONCLUSIVE |
| `SR-STATE-01` | An account flag change needs existing flags absent from the snapshot. A sponsored signer, or an operation in a sponsorship sandwich, is not modelled. A signer is added and the snapshot has no `reserve` data. A `fee` is below 100 stroops for each operation. | INCONCLUSIVE |
| `SR-INTENT-01` | Plan declared `irreversible`; presentation only. | none |

## 8. Output

### 8.1 Fixed limitations (valid text and JSON reports)

> Availability is declared by the user. This tool does not and cannot verify that anyone holds any key.
> The result applies to the account state at ledger N on network X. Authorization happens at apply time against the live state, which may have changed.
> Only the features listed under "coverage" are modelled. Recognized unsupported inputs are reported INCONCLUSIVE.

Rejected-input reports use a shorter form with the `INCONCLUSIVE` verdict and input problems. The text above describes the full report for validated inputs; it is not a literal quote of every output line.

### 8.2 Sections

1. **Verdict** and the single most important reason in one sentence.
2. **Provenance:** network, account, ledger, capture time, source, protocol version. The tool does not calculate snapshot age.
3. **Assumptions:** the availability table and scenarios, as given.
4. **Step trace:** for each transaction, the pre-state, which signatures matched with their weights, the threshold needed per operation, and accepted or rejected. When the snapshot has `reserve` data, also the balance figures before the transaction (balance, minimum balance, available, in stroops) and the fee, with a note when the fee is the assumed 100 stroops for each operation. These lines and the matching JSON fields (`starting_reserve`, `reserve_after`, and `reserve_before` and `fee` on each transaction) are written only when `reserve` data is present. A document without it gets none of them. The coverage list (8.1) changed for every report.
5. **Reachability:** LOW, MEDIUM and HIGH for each scenario, with `W_all`, `W_avail`, `W_unk`.
6. **Diagnostics:** codes with locations.
7. **Coverage:** what was modelled and what was not.
8. **Unsafe reason:** the failing transaction or scenario is named with its diagnostic. The tool does not prove a mathematically minimal counterexample.

### 8.3 Machine-readable output

JSON with a versioned schema (`signerreach.report/1`), a stable ordering of fields and diagnostics, and no timestamps in the verdict payload other than provenance, so that identical input yields byte-identical output.

### 8.4 Declared irreversible lock

If `intent` is `irreversible`, the report states "Irreversible administrative state, declared by user" and still prints the underlying verdict and diagnostics unchanged. It never turns an UNSAFE into SAFE.

## 9. Command line

| Command | Behaviour |
|---|---|
| `signerreach check --snapshot S --plan P --availability A` with optional `--format` set to text or json | Implemented. Evaluate. Exit code 0 SAFE WITHIN SCOPE, 1 UNSAFE, 2 INCONCLUSIVE, 64 usage error. Markdown is issue #2. |
| `signerreach fetch --network N --account G` with optional `--horizon-url U` | Read-only. Prints a snapshot with provenance on standard output and writes no file. Reads from the public Horizon server for the network, or from `U` (https, or http for this machine only). Exit code 0 with a snapshot, 64 for bad arguments, 69 if the account could not be read. No write path exists anywhere in the binary. |
| `signerreach schema KIND` where KIND is snapshot, plan, availability or report | Planned in issue #1; no command exists yet. |

`check` makes no network calls. `fetch` is the only command that does, and it sends nothing but `GET` requests for the server root, one account, and one ledger record. The ledger is the one the account answer is as of (`/ledgers/<as_of_ledger>`), and it is read only for `base_reserve_in_stroops`, so the snapshot's `reserve` block describes one ledger. The ledger record must be for that sequence and for the same protocol version as the server root, or the fetch fails. It follows no redirects, uses no proxy, and limits each request to 30 seconds and each answer to 1 MiB.

## 10. Test corpus

Each fixture has an ID, the three inputs, and the expected verdict, codes and per-transaction outcome. Fixtures marked **chain** are backed by Testnet observations (see `docs/evidence/testnet-2026-10-13/README.md`). Fixtures marked **synthetic** test the tool's input handling and are never to be presented as chain evidence.

| ID | Source | State (master/signers; thresholds low/med/high) | Plan and availability | Expected |
|---|---|---|---|---|
| FX-01 | chain, Case 1 | A:1, B:1; 1/1/1 | tx1 `[high=2]` signed A. A available, B lost | tx1 accepted. **UNSAFE `SR-LOSS-01`** (W_all 2, W_avail 1, HIGH 2) |
| FX-02 | chain, Case 2 | A:1, B:1, C:1; 1/2/2 | one tx `[add D:1, remove C]` signed A,B. A,B,D available, C lost | accepted. final A,B,D. **SAFE WITHIN SCOPE** |
| FX-03 | chain, Case 3 | A:1, B:1; 1/1/2 | tx `[master_weight=0]` signed A,B. All available | accepted. **UNSAFE `SR-LOCK-01`** (total weight 1, HIGH 2) |
| FX-04 | synthetic, Case 4 | final A,B,D weight 1 each; 1/2/2 | A available, B and D unknown | **INCONCLUSIVE `SR-AVAIL-01`** |
| FX-05a | chain, Case 5a | A:1, B:1; 1/2/2 | ONE tx `[remove B, master_weight=2]` signed A,B. All available | accepted. final A:2. **SAFE WITHIN SCOPE** |
| FX-05b | chain, Case 5b | A:1, B:1; 1/2/2 | TWO txs: tx1 `[remove B]` signed A,B; tx2 `[master_weight=2]` signed A. All available | tx1 accepted, **tx2 rejected**. **UNSAFE**: `SR-AUTH-01` at tx2 op 0, and `SR-LOCK-01` because after tx1 the account has total weight 1 against HIGH 2 |
| FX-05c | chain, Case 5c | A:1, no signers; 0/0/0 | ONE tx `[add B:1 and thresholds 2/2/2; set_home_domain]` signed A,B | **rejected, `SR-TX-01`** (B matches no pre-state signer). **UNSAFE** |
| FX-07 | chain, Case 7 | A:1; low 5, medium 0, high 0 | state check, no plan. A available | LOW unreachable, HIGH reachable. **INCONCLUSIVE `SR-TX-03`** |
| FX-06a | synthetic | any | plan contains a `payment` | INCONCLUSIVE `SR-OP-01` |
| FX-06b | synthetic | state has a `pre_auth_tx` signer | any | INCONCLUSIVE `SR-SIGNER-01` |
| FX-06c | synthetic | signer weight 256 | any | INCONCLUSIVE `SR-INPUT-01` |
| FX-06d | synthetic | snapshot without `as_of_ledger` | any | INCONCLUSIVE `SR-FRESH-01` |
| FX-06e | synthetic | `protocol_version` 28 | any | INCONCLUSIVE `SR-RULES-01` |
| FX-06f | synthetic | `account` starts with `C` | any | INCONCLUSIVE `SR-INPUT-02` |
| FX-06g | synthetic | A:2, B:1; 1/2/2 | tx `[set_home_domain]` signed A,B (A alone meets every check) | INCONCLUSIVE `SR-TX-02` |
| FX-08 | synthetic | A:1, B:1; 1/1/2 | empty plan, `require_survive_loss_of: 1`, all available | UNSAFE `SR-LOSS-01` (loss of A or B leaves weight 1) |
| FX-09 | synthetic | A:1, B:1, C:1; 1/2/2 | empty plan. A, B, C available; scenarios `A-lost` and `A-and-B-lost` | UNSAFE `SR-LOSS-01`: safe in the baseline and in `A-lost`, unsafe in `A-and-B-lost`; the worst scenario decides |

**Reserve and fee fixtures (R10 to R14).** All use account A (master 1) with signer B (weight 1), thresholds 1/2/2, `subentry_count` 1, no sponsorship, `base_reserve` 5,000,000, so `minBalance` is 15,000,000. Unless the row says otherwise, the plan is one transaction `[add D:1]` signed A and B, the fee is the assumed 100 stroops, and A, B and D are available. They are `synthetic` in the corpus: the numbers follow from R10 to R14 as read in the source. FX-10c is built from the failure of 2026-11-20, and FX-10a, FX-10b, FX-11a, FX-11b, FX-12, FX-14 and FX-17 were also run with exactly these balances on Testnet on 2026-11-23, where the network did what the table says. FX-15 (no data) was not run on a network. FX-13 (liabilities) and FX-16 (1000 sub-entries) were not run as written, but cases of the same kind were, at other figures, on 2026-11-28: selling liabilities of 1,000,000 stroops, and 1000 sub-entries with a balance of 5,015,001,000 stroops (section 4).

| ID | Source | Balance (stroops) and change from the base | Expected |
|---|---|---|---|
| FX-10a | synthetic | 20,000,100: exactly `minBalance + baseReserve + fee` | tx1 accepted. **SAFE WITHIN SCOPE** |
| FX-10b | synthetic | 20,000,099: one stroop less | tx1 fails when applied. **UNSAFE `SR-RESERVE-01`**, short by 1 |
| FX-10c | synthetic, built from the Testnet failure at ledger 5831809 | A alone (no signers), thresholds 0/0/0, `subentry_count` 0, balance 10,010,000, plan `[add B:1]` signed A | tx1 fails when applied. **UNSAFE `SR-RESERVE-01`**, short by 4,990,100 |
| FX-11a | synthetic | Signers B and C, `subentry_count` 2, balance 20,000,200. Plan `[remove C, add D:1]`, fee 200 | accepted: the removal frees one reserve first (R14). **SAFE WITHIN SCOPE** |
| FX-11b | synthetic | as FX-11a, plan `[add D:1, remove C]` | tx1 fails when applied at operation 0. **UNSAFE `SR-RESERVE-01`** |
| FX-12 | synthetic | 15,000,100, plan `[set B to weight 2]` | accepted: re-weighting needs no reserve (R10). **SAFE WITHIN SCOPE** |
| FX-13 | synthetic | 20,000,100 with `selling_liabilities` 1 | tx1 fails when applied. **UNSAFE `SR-RESERVE-01`**, short by 1 |
| FX-14 | synthetic | 15,000,099, plan `[set B to weight 2]` | tx1 **rejected** (`tx_insufficient_balance`, R13): `available` 99 is below the fee. **UNSAFE `SR-RESERVE-01`** |
| FX-15 | synthetic | FX-10a with no `reserve` object | tx1 accepted as if affordable. **INCONCLUSIVE `SR-STATE-01`** |
| FX-16 | synthetic | `subentry_count` 1000, balance 100,000,000,000 | tx1 fails when applied. **UNSAFE `SR-APPLY-01`** |
| FX-17 | synthetic | 15,000,150, two transactions: tx1 `[add D:1]`, tx2 `[set B to weight 2]` | tx1 fails when applied and keeps its fee (R12), so tx2 is **rejected**: `available` is 50, the fee 100. **UNSAFE `SR-RESERVE-01`** |

FX-02 and the other fixtures that add a signer carry a `reserve` object, because without one they are `INCONCLUSIVE` (FX-15). For the chain fixtures the object is a lower bound, not an observation: the accounts were funded with 10,000 XLM and paid setup fees, so each real balance is higher than the fixture states. Setting it below the real balance can only make the checker more cautious.

**Chain fixtures are replayed, not trusted.** Testnet resets periodically. The repository must carry a separate replay harness (not part of the default CLI) that rebuilds each chain fixture on a fresh test network and compares observed to expected. A fixture whose replay differs from its expectation blocks release.

## 11. Acceptance criteria for V1

V1 is accepted only if all of these hold:

1. Every fixture in section 10 passes, and no unsafe fixture is reported SAFE.
2. A replay of each chain fixture on a fresh Testnet matches the expected outcome, with ledger and hash recorded.
3. The same input always yields byte-identical JSON output.
4. The `check` command opens no network connection. This is shown by tests that fail if the core depends on any networking crate, or if the core or the `check` command names networking code (`crates/signerreach-cli/tests/offline.rs`). It is not shown by watching sockets at run time.
5. The shipped CLI needs no private key, seed or signature. The development replay harness uses temporary Testnet keys in memory and does not print or persist their secrets.
6. Every INCONCLUSIVE path has a test and a named code.
7. The documentation states the fixed header text and the coverage list.
8. No claim of audit, adoption, approval or funding appears in any file.
9. A plan that adds a signer is never reported `SAFE WITHIN SCOPE` when the balance in the snapshot cannot pay for it (FX-10b, FX-10c, FX-11b, FX-13, FX-14, FX-17), and never when the snapshot has no `reserve` data (FX-15).
10. Each of R10 to R14 is recorded on a network at its boundary, with ledger and transaction hash, or this specification says that it is not. The state of that record is stated in section 4.

## 12. Engineering principles

- Each commit is one logical unit, with its tests. No commit exists only to raise activity numbers.
- Compiler, SDK and crate versions are verified on the build date and recorded in the lock file and CI, not copied from this document.
- Issues are opened only from real, evidenced work.
- Documentation claims are limited to what is tested. No file may claim audit, adoption, approval or funding.
- The shipped CLI must not ask for or disclose private signing material. The replay harness must not print or persist its temporary Testnet secrets.

## 13. Glossary

| Term | Meaning |
|---|---|
| Threshold class | LOW, MEDIUM or HIGH requirement for an operation |
| Administrative path | A set of available signers whose weight meets the HIGH threshold |
| Pre-state | The account state before a transaction is applied |
| Scenario | A named set of availability overrides |
| Structural lock | Total weight of all signers is below a threshold, even if every key is held |
| Extra signature | A signature that no check uses; the transaction is rejected |
| Chain fixture | A case verified on a real network |
| Synthetic fixture | A case that tests the tool's own input handling |
