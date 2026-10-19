# Testnet replay, 2026-10-19

A run of [`tools/replay`](../../../tools/replay/README.md) on the seven `chain`
fixtures. Every outcome matched the fixture. This is a second, independent
observation of the rules first recorded in
[`../testnet-2026-10-13`](../testnet-2026-10-13/README.md), made with a fresh set
of throwaway accounts.

## What was run

- Network: Stellar Testnet, protocol 29, `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)`.
- Ledgers: 5280616 (start) to 5280643 (last accepted transaction).
- Client: `@stellar/stellar-sdk` 17.2.1, run by `tools/replay/replay.mjs` as committed in `319eb0b`.
- Run window: 2026-10-19T16:17:47Z to 2026-10-19T16:20:03Z.
- Accounts: throwaway keys generated at run time and funded by Friendbot. Secret
  keys were held in memory only. Nothing was submitted to mainnet.

The core binary is version 30 while the network still runs protocol 29. The
rules checked here are the protocol 29 rules, and they held on this newer core.
This run says nothing about protocol 30 itself, which was not live.

## Files

| File | Content |
|---|---|
| `results.json` | The raw output of the run, unmodified. |
| `README.md` | This summary. |

## Results

Each fixture's account was first set to the fixture's snapshot (`setup`), read
back, and checked against it. Then the plan's transactions were submitted with the
signatures the plan names.

| Fixture | Step | Account | Observed | Transaction hash |
|---|---|---|---|---|
| FX-01 | setup | `GDZ5LY..TYRO` | accepted, ledger 5280620 | `741076cf98e0bed6e1fb6619560678912dc35bc571d1a603f025357fe03df42d` |
| FX-01 | raise-high | `GDZ5LY..TYRO` | accepted, ledger 5280621 | `30c3b9a24bb49029adcb2daad1028b4aff0bb71df8c9964d3976953ff08c408f` |
| FX-02 | setup | `GBUWAY..HQ2S` | accepted, ledger 5280623 | `303e5d439f9f07ab93529d50d9813dbafdc002eacb9fcc27534b50f29fc40fda` |
| FX-02 | rotate | `GBUWAY..HQ2S` | accepted, ledger 5280624 | `2f13734a259ca4d35c516fecaeaa8247e9e3030a5596c0b510aa870a0d33ce63` |
| FX-03 | setup | `GCEZM7..XO2L` | accepted, ledger 5280627 | `bdfbedeba0ffd03317f3b5b7608849eb447fb6849f191a1a1a4003abb1b02bdd` |
| FX-03 | zero-master | `GCEZM7..XO2L` | accepted, ledger 5280628 | `06670d6469d7943ed80b0d3885e74348e32e13f11ce64efe21d880eaf27c9cf0` |
| FX-05a | setup | `GAS4RG..D7PK` | accepted, ledger 5280632 | `84d66f1c9353ea8d878202ee28043b4c0c6faa371fdcfa69173ca09e31546985` |
| FX-05a | one | `GAS4RG..D7PK` | accepted, ledger 5280633 | `f60c7c1dba79136691547cc817e2b835a3f719621432835ec876c25a003f662b` |
| FX-05b | setup | `GA33R6..NDZO` | accepted, ledger 5280637 | `2390a381c4464ff74d7690fd944df0ebb4ec6025e9e68810ae3ac00310a437f9` |
| FX-05b | first | `GA33R6..NDZO` | accepted, ledger 5280638 | `c0fe75d9afd5f570767b67e5329a06419139a89b5f7d22eecfe0a75e00f5d610` |
| FX-05b | second | `GA33R6..NDZO` | rejected: tx_failed / op_bad_auth | `3f6a2f3d060f5a5f25afffd9af66917e9629976fdcc3cc1e17c4be45c2b26f19` (computed, not in a ledger) |
| FX-05c | add-b | `GDEG3O..P6KU` | rejected: tx_bad_auth_extra | `fffba386bce60824f2127db1ebdfaf99af4812c256fbd7444c15723d52390286` (computed, not in a ledger) |
| FX-07 | setup | `GBNLUB..SOVP` | accepted, ledger 5280643 | `bb60afd2bc801e57f572d5cfdc3307ce680eb6849d1064cbd34a6862f359a37b` |
| FX-07 | probe | `GBNLUB..SOVP` | rejected: tx_bad_auth | `c15614769772a4bf57c823ec0ba660e141f50d201e1ed9a029bb275256470f4b` (computed, not in a ledger) |

What each fixture expected, and so what matched:

- FX-01, FX-02, FX-03, FX-05a: the plan's one transaction is accepted.
- FX-05b: the first transaction is accepted and the second is rejected. FX-05a and
  FX-05b make the same two changes, as one transaction and as two.
- FX-05c: the transaction is rejected for an unused signature.
- FX-07: a change-nothing transaction signed by every signer is rejected, because
  the account's LOW threshold (5) is above its total weight (1). This row is a
  probe whose expectation comes from the stated rule, not from `expected.json`.

## Notes

- **Rejected transactions are not in any ledger.** They consumed no fee and no
  sequence number, and their hashes are computed, not looked up.
- **The tool records every refusal as `rejected`.** It does not tell a rejection
  at submission from a failure to apply. These fixtures expect only `accepted`
  and `rejected`.
- **What this run does not cover.** It does not repeat the follow-up attempts of
  the 2026-10-13 evidence, such as trying to change a threshold after a plan that
  locked the account. The lockout claims of FX-01 to FX-03 rest on that earlier
  evidence.
