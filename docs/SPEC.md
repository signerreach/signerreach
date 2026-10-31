# SignerReach V1 Specification (draft 0.1)

Date: 2026-10-14
Status: Draft 0.1, accepted as the basis for the V1 implementation. **Nothing described here is implemented yet.** A behaviour in this document counts as delivered only when the matching test in section 10 passes.

---

## 1. Purpose in one paragraph

SignerReach is an offline command-line checker. Before a person submits a change to the signers or thresholds of a Stellar classic account, it takes a snapshot of the account, an ordered plan of changes, and a declaration of which signers are available, and it answers one question: **after this plan, can the account still be administered, under the stated assumptions?** It answers SAFE WITHIN SCOPE, UNSAFE or INCONCLUSIVE and shows the authorization trace that led to the answer. It never holds keys, signs, or submits anything.

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
- **Network and rules:** Stellar network at **protocol 29** only (the version on which the rules were verified).
- **Signer types:** `ed25519` keys, including the account's own master key.
- **Operations in a plan:** `SetOptions` only. A `SetOptions` is *administrative* when it sets master weight, a threshold, or a signer. A home-domain-only or empty `SetOptions` needs MEDIUM authorization. Flag changes cannot be decided from this snapshot format.
- **Transaction source:** the analysed account itself.
- **Availability:** each signer is `available`, `lost` or `unknown`. Default is `unknown`.
- **Outputs:** text, JSON and Markdown reports.

### 3.2 Out of scope (always INCONCLUSIVE with a named code)

Soroban or contract accounts (`C...`), muxed accounts, fee-bump transactions, transaction preconditions beyond what is listed, operation or transaction source accounts other than the analysed account, account flag changes, sponsorship and reserve effects, `pre_auth_tx`, `hash_x` and `ed25519_signed_payload` signers, any operation type other than `SetOptions` (including `AccountMerge`, which needs HIGH but ends the account and is not modelled as a plan step), any protocol other than 29.

### 3.3 Non-goals

Key collection, signing, broadcasting, custody, fee optimization, payments, asset issuance, a frontend, a backend service, a database, a dashboard, a token, a smart contract, predicting human behaviour.

## 4. Verified rules this specification relies on

Each rule below was read in stellar-core source (commit `66defff`) and observed on Stellar Testnet at protocol 29. The evidence is in `docs/evidence/testnet-2026-10-13/`. Rule identifiers are used in test fixtures.

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
  "sponsored_signers": false
}
```

- `provenance.as_of_ledger` and `network` are **required**. Without them the verdict is INCONCLUSIVE (`SR-FRESH-01`).
- The master key is represented only by `master_weight`, not as a signer entry. A snapshot adapter that reads Horizon must remove the master key from the signer list it returns.
- `sponsored_signers: true` makes the run INCONCLUSIVE (`SR-STATE-01`).

### 5.2 Plan

```json
{
  "schema": "signerreach.plan/1",
  "transactions": [
    {
      "id": "tx1",
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
- `signed_by` lists the keys that will sign that transaction. It is required because authorization depends on it.
- A `set_options` operation may carry any of: `master_weight`, `low`, `medium`, `high`, `signer`, `home_domain`, `set_flags`, `clear_flags`. Only `master_weight`, the three thresholds and `signer` change the model. A flag change is parsed but makes the transaction undecidable with `SR-STATE-01`, because the snapshot omits existing account flags. A valid home domain does not change the modeled signer state.
- `transactions` may be empty. An empty plan is a state-only check of the snapshot.
- Any other operation type, including `account_merge`, is rejected with `SR-OP-01`.
- `id` is 1 to 64 characters from letters, digits, `.`, `_` and `-`, and unique within the plan.
- `signed_by` holds at most 20 distinct `G...` keys (a Stellar limit). A key of another kind is `SR-SIGNER-01`.
- A transaction holds 1 to 100 operations (a Stellar limit) and a plan at most 1000 transactions (a limit of this tool).
- A transaction or an operation may carry `source`, a `G...` address. It is read and kept; a source other than the analysed account is reported as `SR-TX-04` when the plan is evaluated against a snapshot.
- A transaction that carries `fee_bump` or `preconditions` is `SR-TX-04`; their content is not read.
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

Reject (INCONCLUSIVE) on: unknown fields; weights or thresholds outside 0 to 255; duplicate signer keys; a signer entry with weight 0 (a weight of 0 removes a signer, so it cannot appear in a state); more than 20 additional signers in any state; more than 20 entries in `signed_by`; a signer key equal to the account; a non-G key; missing provenance; protocol other than 29; any item listed in 3.2. Each rejection carries the matching diagnostic code and the exact location in the input.

### 6.2 Step 2: authorize each transaction (rules R2 to R7)

For transaction *T* against pre-state *S*:

If any operation changes account flags, the transaction is undecidable (`SR-STATE-01`). Stellar can refuse such a change based on flags absent from the snapshot.

1. **Matching.** Each key in `signed_by` is matched to the account's master key (if master weight above 0) or to an additional signer in *S*. A key that matches nothing is *extra* → `SR-TX-01`, transaction rejected.
2. **Envelope check.** Sum of matched weights (each capped at 255, each key counted once) must be at least `S.low`, and at least one key must match. Otherwise `SR-AUTH-01`.
3. **Operation checks.** For each operation in *T* in order, determine its threshold class with R4 using **S, not intermediate state**. The matched sum must be at least that threshold. Otherwise `SR-AUTH-01` with the failing operation index.
4. **Necessity of signatures (R7).** For each key in `signed_by`, test whether removing it leaves at least one check failing. If every key is necessary, all are used and the transaction is valid. If some key is not necessary, the outcome depends on signature order: the transaction is reported as `SR-TX-02` and the run cannot be SAFE (it becomes INCONCLUSIVE).
5. If any step fails, the transaction is **rejected**. The state is unchanged (R8). Evaluation of later transactions continues against the unchanged state so the report can show every failure, but the verdict is already UNSAFE.

If the transaction is valid, apply its operations **in order** to produce the next state. Applying an operation that removes a signer that does not exist is a no-op. Setting a signer weight to 0 removes it.

An operation that cannot be applied makes the whole transaction fail when applied: it names the account's own key as a signer (even to remove it), or it would take the account above 20 additional signers. This is `SR-APPLY-01`: the state is unchanged and the verdict is UNSAFE. Operations are applied one after another, so at the limit a removal before an addition fits where the reverse order does not.

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
| Any transaction rejected in 6.2 | UNSAFE | `SR-AUTH-01`, `SR-TX-01`, `SR-APPLY-01` |
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

A false SAFE can cost someone their account. Wherever a rule is not verified, the result is INCONCLUSIVE. An INCONCLUSIVE result is a correct answer.

## 7. Diagnostic codes

The proposal book's code names were explicitly "proposals, not an established API". They are fixed here for V1 with the meanings below. `SR-TX-01` changed meaning from the proposal.

| Code | Meaning | Verdict effect |
|---|---|---|
| `SR-AUTH-01` | A transaction or operation is not authorized by the state before that transaction. Names the earliest failing transaction and operation. | UNSAFE |
| `SR-TX-01` | A signature in `signed_by` matches no signer of the pre-transaction state (would be rejected `tx_bad_auth_extra`). | UNSAFE |
| `SR-TX-02` | A signature is not necessary; whether it is rejected as extra depends on signature order. | INCONCLUSIVE |
| `SR-TX-03` | LOW threshold unreachable; administration requires another funded transaction source. | INCONCLUSIVE |
| `SR-TX-04` | Fee-bump, preconditions, or a source account other than the analysed account. | INCONCLUSIVE |
| `SR-APPLY-01` | The transaction is authorized but would fail when applied: it names the account's own key as a signer, or it would give the account more than 20 additional signers. | UNSAFE |
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
| `SR-STATE-01` | An account flag change needs existing flags absent from the snapshot. Sponsorship and reserve conditions are not modelled. | INCONCLUSIVE |
| `SR-INTENT-01` | Plan declared `irreversible`; presentation only. | none |

## 8. Output

### 8.1 Fixed header (always printed, never omitted)

> Availability is declared by the user. This tool does not and cannot verify that anyone holds any key.
> The result applies to the account state at ledger N on network X. Authorization happens at apply time against the live state, which may have changed.
> Only the features listed under "coverage" are modelled. Anything else is reported INCONCLUSIVE.

### 8.2 Sections

1. **Verdict** and the single most important reason in one sentence.
2. **Provenance:** network, account, ledger, capture time, source, rule version, snapshot age.
3. **Assumptions:** the availability table and scenarios, as given.
4. **Step trace:** for each transaction, the pre-state, which signatures matched with their weights, the threshold needed per operation, and accepted or rejected.
5. **Reachability:** LOW, MEDIUM and HIGH for each scenario, with `W_all`, `W_avail`, `W_unk`.
6. **Diagnostics:** codes with locations.
7. **Coverage:** what was modelled and what was not.
8. **Counterexample** for UNSAFE: the smallest declared scenario and signer set that fails.

### 8.3 Machine-readable output

JSON with a versioned schema (`signerreach.report/1`), a stable ordering of fields and diagnostics, and no timestamps in the verdict payload other than provenance, so that identical input yields byte-identical output.

### 8.4 Declared irreversible lock

If `intent` is `irreversible`, the report states "Irreversible administrative state, declared by user" and still prints the underlying verdict and diagnostics unchanged. It never turns an UNSAFE into SAFE.

## 9. Command line

| Command | Behaviour |
|---|---|
| `signerreach check --snapshot S --plan P --availability A` with optional `--format` set to text, json or markdown | Evaluate. Exit code 0 SAFE WITHIN SCOPE, 1 UNSAFE, 2 INCONCLUSIVE, 64 usage error. |
| `signerreach fetch --network N --account G` with optional `--horizon-url U` | Read-only. Prints a snapshot with provenance on standard output and writes no file. Reads from the public Horizon server for the network, or from `U` (https, or http for this machine only). Exit code 0 with a snapshot, 64 for bad arguments, 69 if the account could not be read. No write path exists anywhere in the binary. |
| `signerreach schema KIND` where KIND is snapshot, plan, availability or report | Print the JSON Schema. |

`check` makes no network calls. `fetch` is the only command that does, and it sends nothing but `GET` requests for the server root and one account. It follows no redirects, uses no proxy, and limits each request to 30 seconds and each answer to 1 MiB.

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

**Chain fixtures are replayed, not trusted.** Testnet resets periodically. The repository must carry a separate replay harness (not part of the default CLI) that rebuilds each chain fixture on a fresh test network and compares observed to expected. A fixture whose replay differs from its expectation blocks release.

## 11. Acceptance criteria for V1

V1 is accepted only if all of these hold:

1. Every fixture in section 10 passes, and no unsafe fixture is reported SAFE.
2. A replay of each chain fixture on a fresh Testnet matches the expected outcome, with ledger and hash recorded.
3. The same input always yields byte-identical JSON output.
4. The `check` command opens no network connection. This is shown by tests that fail if the core depends on any networking crate, or if the core or the `check` command names networking code (`crates/signerreach-cli/tests/offline.rs`). It is not shown by watching sockets at run time.
5. No private key, seed or signature is read, stored or logged anywhere.
6. Every INCONCLUSIVE path has a test and a named code.
7. The documentation states the fixed header text and the coverage list.
8. No claim of audit, adoption, approval or funding appears in any file.

## 12. Engineering principles

- Each commit is one logical unit, with its tests. No commit exists only to raise activity numbers.
- Compiler, SDK and crate versions are verified on the build date and recorded in the lock file and CI, not copied from this document.
- Issues are opened only from real, evidenced work.
- Documentation claims are limited to what is tested. No file may claim audit, adoption, approval or funding.
- No code path may read, store or log a private key, seed or signature.

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
