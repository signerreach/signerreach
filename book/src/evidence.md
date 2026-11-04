# Protocol 29 and 30 evidence

SignerReach's supported account-authorization rules are backed by source analysis, offline tests and Stellar Testnet observations. **The October and early November records are Protocol 29 evidence. The November 4 record is Protocol 30 evidence.**

| Date | Scope | Record |
| --- | --- | --- |
| October 13, 2026 | Original `SetOptions`, signer/threshold and rejection experiments | [Raw result and explanation](https://github.com/signerreach/signerreach/blob/main/docs/evidence/testnet-2026-10-13/README.md) |
| October 19, 2026 | Seven chain-backed fixture plans, without repeating the original follow-up attempts | [Replay record](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-10-19/README.md) |
| November 1, 2026 | Fresh-account FX-01 Case 1b: accepted HIGH change, then rejected master-only follow-up | [Follow-up evidence](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-11-01-fx-01-followup/README.md) |
| November 4, 2026 | The seven chain fixtures and the FX-01 follow-up, replayed on a Testnet running **Protocol 30**; every outcome matched | [Protocol 30 replay](https://github.com/signerreach/signerreach/blob/main/docs/evidence/replay-2026-11-04-protocol-30/README.md) |

## FX-01: a lockout risk demonstrated

In the November 1 replay, the plan raised HIGH to 2 and was accepted in **ledger 5,500,524**. The later `SetOptions(high=0)`, signed only by the available master, was rejected at submission with HTTP **400**, transaction code `tx_failed` and operation code `op_bad_auth`.

The replay reconciled the account sequence, two weight-1 signers and thresholds before and after the rejection. Five bounded history checks did not find the rejected hash. **Bounded absence is not absolute proof of non-inclusion**, and the record does not claim it is.

The checker itself does not submit any transaction; only the separate development replay harness submits to Testnet. Other historical follow-up scenarios have not yet been added to that repeatable replay path.

## What the evidence does not show

These observations do **not** verify the state of accounts today, possession of keys, Mainnet execution, or every authorization edge case. The Protocol 30 record covers only those seven fixtures and one follow-up attempt on Testnet; Mainnet was still on Protocol 29 when it was made. The [evidence and public-claim ledger](https://github.com/signerreach/signerreach/blob/main/evidence/index.md) distinguishes verified results from assumptions and remaining limitations.
