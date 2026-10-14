# Testnet evidence, 2026-10-13

This folder records an experiment that checked how Stellar classic-account authorization behaves for changes to signers and thresholds. It supports the rules R1 to R9 in `docs/SPEC.md`.

## What was run

- Network: Stellar Testnet, protocol 29, `stellar-core 29.0.0 (4eb83337380a29ad0907f4e32196ce97b8dc7649)`.
- Ledgers: 5180915 (start) to 5180938 (last accepted transaction).
- Client: `@stellar/stellar-sdk` 17.2.1.
- Run window: 2026-10-13T21:49:21.116Z to 2026-10-13T21:51:17.617Z.
- Accounts: throwaway keys generated at run time, funded by Friendbot. Secret keys were held in memory only and were never written to disk. Nothing was submitted to mainnet.

## Files

| File | Content |
|---|---|
| `run-cases.js` | The script exactly as it was run. |
| `results.json` | Raw output of that run. |
| `README.md` | This summary. |

## Results

Predicted outcomes were written down before the run, from reading stellar-core source and its own tests. In every row below the observation matched the prediction.

| Case | Account | Observed | Transaction hash |
|---|---|---|---|
| 1a | `GCC647..W5UE` | accepted, ledger 5180919 | `261eedc57d15c5b00c31eb2f94eb6ef1bb86afb1fa78fb668f5787ed28156f00` |
| 1b | `GCC647..W5UE` | rejected at submission: tx_failed / op_bad_auth | `0c16c50b66559c3936ff37337fc384c595f7858290166db50246b960e519608e` (computed, not in a ledger) |
| 1c | `GCC647..W5UE` | accepted, ledger 5180920 | `2d77d1e05dc4a10e7d3b67ce87a4e5f5f43c852c3bb0ce42754141b1564bd778` |
| 2a | `GDIIIH..XGZO` | accepted, ledger 5180923 | `a973a174480138e8a11d8448d9702127592e0e4d1e8030e7dea8aa966f0b31f6` |
| 2b | `GDIIIH..XGZO` | accepted, ledger 5180924 | `1509265ba2fbce758fe0fe042aeec2dd478efae3d3a1f8b1654c4955c452d853` |
| 2c | `GDIIIH..XGZO` | rejected at submission: tx_failed / op_bad_auth | `92390cda36718a17ae2c884256ad2bf3107ac7747a4defa509fd179d7e5d478a` (computed, not in a ledger) |
| 3a | `GCCZMO..VXRA` | accepted, ledger 5180927 | `287fb5bf07cc54572c4478a03e8811b80cd825aad6d8c7fb739ff5d3cae8f22b` |
| 3b | `GCCZMO..VXRA` | rejected at submission: tx_failed / op_bad_auth | `a644801aec61ea0cdeaf95f8175f238f57bca9028a5c53d5f0ab430a06dd8bbe` (computed, not in a ledger) |
| 5a | `GAZOPG..BLEU` | accepted, ledger 5180930 | `459148158da7fff211bb0ac1fb507d8945c8e824d2ddde58b86bafb5176048d8` |
| 5b-1 | `GD7OXE..OGH5` | accepted, ledger 5180933 | `7968546df5e17d3e90119c8d5bf56c7f959b102ca183c17c0bfb1dcd8dab4487` |
| 5b-2 | `GD7OXE..OGH5` | rejected at submission: tx_failed / op_bad_auth | `8c4b56097fddf6420257f8616f93cacdda1950bf74d751ff9770ad257e2746eb` (computed, not in a ledger) |
| 5c | `GCW2KZ..KVQZ` | rejected at submission: tx_bad_auth_extra | `3e9237ae5d2e4fd312b1eff06db507b7f0d88cd35d9f58c9d653951a544ef411` (computed, not in a ledger) |
| 7a | `GCPY7L..3LDS` | accepted, ledger 5180937 | `edb786dc08f665994cd88cf8dc467fac6294ccf431bf4eef58be572e80645d84` |
| 7b | `GCPY7L..3LDS` | rejected at submission: tx_bad_auth | `12ac51255e719bbf1d87b562e33c40cf94981caffcfca37f8738bc15cd789558` (computed, not in a ledger) |
| 7c | `GCPY7L..3LDS` | accepted, ledger 5180938 | `1f3ef604f80ffc5e48f04c813d392584f239d952e862f6e840548bda1caa7f18` |

## What the cases show

- **1b, 3b:** after a plan that leaves too little reachable weight for the HIGH threshold, any further signer or threshold change is rejected, even when the remaining signer is available (3b).
- **2a, 2b, 2c:** a one-transaction rotation `[add D, remove C]` is accepted, the new signer set can then reach HIGH, and a removed signer's signature does not help.
- **5a versus 5b-2:** the same two changes are accepted as one transaction and rejected as two. All operations in a transaction are authorized against the state before that transaction.
- **5c:** a transaction that adds a signer and raises thresholds, signed by the new signer, is rejected `tx_bad_auth_extra`, because the signature is judged against the thresholds before the transaction.
- **7a to 7c:** a LOW threshold the account cannot meet blocks it as a transaction source, and is recoverable by using a different funded account as the transaction source.

## Notes on reading the raw data

- **Rejected transactions are not in any ledger.** Cases marked "rejected at submission" were refused by the validator before inclusion. They consumed no fee and no sequence number, and a lookup of their hash on Horizon returns 404.
- **Hash encoding in `results.json`.** For rejected transactions the script stored the hash as a comma-separated list of decimal bytes (a bug in `run-cases.js`: it called `toString("hex")` on a byte array). The table above shows the same bytes converted to hex. The script and the raw file are left unmodified on purpose.
- **Case 5a-state** in the raw file is a state snapshot, not a transaction.

## What this evidence does not cover

One run per case, on one network, at one protocol version. It does not cover fee-bump transactions, sponsored signers, `pre_auth_tx`, `hash_x` or signed-payload signers, plans with several source accounts, or other protocol versions. Testnet is reset periodically, so the hashes above will stop resolving; rerunning the script creates new accounts.

## Source references

Read at stellar-core `master`, commit `66defff52e126a7ccdf50563f70f53782b73ed4f` (2026-10-13).

| Rule in the specification | Where it was read |
|---|---|
| HIGH only when SetOptions sets master weight, a threshold or a signer | `src/transactions/SetOptionsOpFrame.cpp`, `getThresholdLevel` |
| AccountMerge needs HIGH; trust-flag operations LOW; default MEDIUM | `MergeOpFrame.cpp`, `TrustFlagsOpFrameBase.cpp`, `OperationFrame.cpp` |
| Envelope needs the source account's LOW threshold | `TransactionFrame.cpp`, `checkAllTransactionSignatures` |
| All operation signatures are checked before any operation is applied | `TransactionFrame.cpp`, `processSignatures`, `commonPreApply`; `OperationFrame.cpp`, `checkValid` |
| Threshold met when summed distinct weights are at least the need; weight capped at 255 | `SignatureChecker.cpp`, `checkSignature` |
| Unused signature rejects the transaction | `SignatureChecker.cpp`, `checkAllSignaturesUsed` |
| Behaviour of several operations changing signers in one transaction | `src/transactions/test/TxEnvelopeTests.cpp`, section "change signer and weights mid-transaction" |
