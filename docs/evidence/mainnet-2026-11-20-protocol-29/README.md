# Mainnet state-only checks at protocol 29, 2026-11-20

A run of `signerreach fetch` and `signerreach check` on three public Mainnet
accounts, taken while Mainnet was still on protocol 29 and before the protocol 30
vote. It is a baseline. The same script is meant to be run again after the
network changes protocol, and the two records compared. If the vote does not
happen, this record stands alone.

This run reads data and signs nothing. It is not a replay and it submits no
transaction. The Testnet evidence in [`../testnet-2026-10-13`](../testnet-2026-10-13/README.md),
[`../replay-2026-10-19`](../replay-2026-10-19/README.md) and
[`../replay-2026-11-04-protocol-30`](../replay-2026-11-04-protocol-30/README.md)
stays unchanged.

## What was run

- Binary: `signerreach 0.1.1`, built from commit
  `ee06d8b933f12e6a4f738aa6a0c159a6a475a355` (`main` after pull request #47)
  with `cargo build --release --locked -p signerreach-cli`, using
  `rustc 1.98.0 (88d9e12ae 2026-08-18)` on Windows.
- Network: Stellar Mainnet, `Public Global Stellar Network ; September 2015`,
  read from `https://horizon.stellar.org`. Horizon reported version
  `30.0.1-6f7cff3903be1df4fb980a12910a1313c5d7d8dd` and core
  `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)`.
- Protocol: `current_protocol_version` was **29** in the server root read at the
  start of the run and again at the end (`horizon/root-before.json`,
  `horizon/root-after.json`). The network software was already version 30; the
  protocol was not.
- Ledgers: 65577896 (server root at the start) to 65577898 (server root at the end).
- Run window: 2026-11-20T11:52:07Z to 2026-11-20T11:52:17Z.
- Requests: `GET` only, to the server root and to one account record each. No
  key, seed or signature was used anywhere, and nothing was submitted.

## The accounts

Three public accounts were chosen for variety: a multi-signer account whose own
key has weight 0, an account with a high signer weight and a high threshold, and
a single-key account. They are third parties. This run says nothing about their
operators, and no change to any of them was evaluated. `home_domain` is what the
account itself publishes on the ledger (`horizon/<account>.account.json`); it is
shown only to make the rows easy to recognise.

| Account | `home_domain` on the ledger | Own-key weight | Other signers (weights) | Thresholds low / medium / high | As of ledger |
|---|---|---|---|---|---|
| `GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` | `circle.com` | 0 | 4 (1, 1, 1, 1) | 2 / 2 / 2 | 65577896 |
| `GBVOL67TMUQBGL4TZYNMY3ZQ5WGQYFPFD5VJRWXR72VA33VFNL225PL5` | `stellarport.io` | 1 | 7 (1, 1, 1, 1, 1, 1, 10) | 0 / 0 / 10 | 65577897 |
| `GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX` | `stablecoin.anchorusd.com` | 1 | none | 0 / 0 / 0 | 65577897 |

## What each check asked

Each snapshot came from `signerreach fetch --network pubnet --account <address>`.
Each was then checked with an **empty plan** (`inputs/plan-empty.json`). An empty
plan is a state-only check of the snapshot: no change is proposed. Two declared
assumptions were used:

- **Assumption A, `availability-all-available.json`:** every signer is
  `available`. This is a hypothetical, stated so that the check has something to
  work with. It is not a claim that anyone holds any key. The tool says this itself
  in every report.
- **Assumption B, `availability-unknown.json`:** nothing is declared, so every
  signer is `unknown`.

## Results

| Account | Assumption A | Assumption B |
|---|---|---|
| `GA5ZSEJY..KZVN` | `SAFE WITHIN SCOPE`, exit 0 | `INCONCLUSIVE`, exit 2: reaching the HIGH threshold 2 needs signers whose availability is unknown |
| `GBVOL67T..5PL5` | `SAFE WITHIN SCOPE`, exit 0 | `INCONCLUSIVE`, exit 2: reaching the HIGH threshold 10 needs signers whose availability is unknown |
| `GDUKMGUG..YLEX` | `SAFE WITHIN SCOPE`, exit 0 | `INCONCLUSIVE`, exit 2: reaching the HIGH threshold 0 needs signers whose availability is unknown |

All three `fetch` calls exited 0 and wrote nothing to standard error. Every
`check` call wrote nothing to standard error. Every exit code is in
`exit-codes.tsv`.

## What this shows, and what it does not

It shows that `fetch` and `check` work on live Mainnet records at protocol 29,
that the records carry the protocol and ledger they were read at, and that the
tool declines to give a verdict when nothing is declared about who can sign.

It does not show that anyone holds, or can use, any of these keys. It does not
show that any of these accounts is safe, or unsafe, in any wider sense. Assumption
A is hypothetical. It does not test the authorization rules at protocol 30. That
is what the second run is for.

## Files

| File | Content |
|---|---|
| `run.sh` | The script that produced everything below. SHA-256 `7e699c53197f909e` (first 16 characters). |
| `run-info.txt` | Binary version and the run's start and end times. |
| `inputs/` | The empty plan and the two availability documents. |
| `snapshots/<account>.json` | The output of `fetch`, unmodified. `.stderr` files are empty. |
| `reports/<account>.<assumption>.txt` and `.json` | The output of `check`, in both formats. |
| `horizon/` | The server root before and after, and each account's Horizon record. |
| `exit-codes.tsv` | The exit code of every command. |
| `SHA256SUMS` | Checksums of every file the script produced. This README is not included. |

## Reproduce it

```bash
bash run.sh /path/to/signerreach /path/to/output-directory
```

Anyone running it later will get later ledgers, and may get different signers if
an account changed. Compare the snapshot's `as_of_ledger` and `protocol_version`
first.
