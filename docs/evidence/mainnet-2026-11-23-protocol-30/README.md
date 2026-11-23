# Mainnet state-only checks at protocol 30, 2026-11-23

The second run of the script recorded in
[`../mainnet-2026-11-20-protocol-29`](../mainnet-2026-11-20-protocol-29/README.md),
taken after Mainnet moved from protocol 29 to protocol 30. The earlier record stays
unchanged and is the baseline for this one.

This run reads data and signs nothing. It is not a replay and it submits no
transaction.

## What was run

`run.sh` here is a byte-for-byte copy of the baseline's (SHA-256
`7e699c53197f909ef1f06c57e4456e4b9deba083d95b64938b41b42e838b0ea2` for both). It was run
twice, once per binary, into two folders:

| Folder | Binary | SHA-256 | Why |
|---|---|---|---|
| `old-binary/` | `signerreach 0.1.1` from commit `ee06d8b933f12e6a4f738aa6a0c159a6a475a355`, the binary used for the baseline | `8027312a6a9a78931b7cf31edfee4de1965af00ed26e24c0e25f09b7b153480d` | A like-for-like comparison: only the network changed. |
| `new-binary/` | `signerreach 0.1.1` from commit `70aaaba5a4c906d1bb20f03fd5235581af423257` (the merge of pull request #53, with the reserve and fee rules) | `e5f44450ce034738b0f820d9e86f72972252fe83219838ab3d3b2ac458df3e54` | Shows what the current tool reads from the same accounts. |

- Network: Stellar Mainnet, read from `https://horizon.stellar.org`. Horizon
  `30.0.1-6f7cff3903be1df4fb980a12910a1313c5d7d8dd`, core
  `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)`. These are the same
  software versions the baseline reported.
- Protocol: `current_protocol_version` was **30** in the server root at the start
  and at the end of both runs (`*/horizon/root-before.json`, `*/horizon/root-after.json`).
  In the baseline it was 29.
- Ledgers: `old-binary` 65636941 to 65636943, `new-binary` 65636944 to 65636945
  (server root at the start and at the end). The baseline was 65577896 to 65577898.
- Run windows: `old-binary` 2026-11-23T21:52:38Z to 21:52:47Z, `new-binary`
  21:52:50Z to 21:53:00Z.
- Requests: `GET` only, to the server root and to one account record each. No key,
  seed or signature was used anywhere, and nothing was submitted.
- The binaries were built from clean commits on Windows with
  `cargo build --release --locked -p signerreach-cli`. `new-binary` used
  `rustc 1.98.0`; the build of `old-binary` is described in the baseline.

The same three third-party accounts were used. This run says nothing about their
operators.

## What changed between the two dates

Compared with the baseline, for each of the three accounts:

| | Baseline (2026-11-20, protocol 29) | `old-binary` (2026-11-23, protocol 30) | `new-binary` (2026-11-23, protocol 30) |
|---|---|---|---|
| Protocol in the snapshot | 29 | 30 | 30 |
| Signers, weights, thresholds, own-key weight | as in the baseline README | identical | identical |
| Verdict, assumption A (all available) | `SAFE WITHIN SCOPE`, exit 0 | same, exit 0 | same, exit 0 |
| Verdict, assumption B (nothing declared) | `INCONCLUSIVE`, exit 2 | same, exit 2 | same, exit 2 |
| Exit code of all 15 steps | | identical to the baseline | identical to the baseline |
| `reserve` object in the snapshot | none | none | present (below) |

How each of these was checked, from the files in this folder:

- For every account, the snapshot with its `provenance` removed is identical to the
  baseline's. For `new-binary`, the same holds once the `reserve` object is also
  removed. So the signers, weights and thresholds did not change between the dates.
- The `exit-codes.tsv` of both folders is identical to the baseline's.
- The text and JSON reports of `old-binary` differ from the baseline's only on the
  lines that state the ledger, the capture time and the protocol (`diff` shows
  nothing else, for all three accounts and both assumptions).
- All `.stderr` files are empty.

What the `new-binary` snapshots add, taken from Horizon at the ledger shown in each
snapshot and printed in each report:

| Account | Balance (stroops) | Sub-entries | Selling liabilities | Base reserve |
|---|---|---|---|---|
| `GA5ZSEJY..KZVN` | 321,182,208,225 | 6 | 0 | 5,000,000 |
| `GBVOL67T..5PL5` | 35,200,483,903 | 7 | 0 | 5,000,000 |
| `GDUKMGUG..YLEX` | 40,429,224,663 | 0 | 0 | 5,000,000 |

An empty plan adds no signer, so the reserve data does not change any verdict. The
`new-binary` reports differ from the `old-binary` ones by a block of balance
figures and by the "not modelled" lines, which now name the fee and reserve rules
the tool applies and the ones it does not.

## What this shows, and what it does not

It shows that `fetch` and `check`, in both the baseline binary and the current one,
read live Mainnet records at protocol 30 and give the same verdicts as they gave at
protocol 29 for these three accounts under these two assumptions. It shows that the
protocol number the tool records follows the network (29, then 30).

It does not show that the authorization rules are the same at protocol 29 and 30 in
general. The authorization behaviour at protocol 30 was observed on Testnet, in
[`../replay-2026-11-04-protocol-30`](../replay-2026-11-04-protocol-30/README.md) and
[`../testnet-2026-11-23-reserve-boundaries`](../testnet-2026-11-23-reserve-boundaries/README.md).
It does not show that anyone holds, or can use, any of these keys, or that any of
these accounts is safe or unsafe in a wider sense. Assumption A is hypothetical.
Three accounts on one day are a small sample. It does not say when Mainnet moved to
protocol 30: both runs began after it had.

## Files

| File | Content |
|---|---|
| `run.sh` | The baseline's script, unchanged. |
| `old-binary/`, `new-binary/` | Each holds what the script produced: `run-info.txt`, `inputs/`, `snapshots/`, `reports/`, `horizon/`, `exit-codes.tsv` and `SHA256SUMS`. |

## Reproduce it

```bash
bash run.sh /path/to/signerreach /path/to/output-directory
```

Anyone running it later will get later ledgers, and may get different signers or
balances if an account changed.
