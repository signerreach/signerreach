# Testnet replay at protocol 30, 2026-11-04

A run of [`tools/replay`](../../../tools/replay/README.md) on the seven `chain`
fixtures, on a Testnet that had moved to protocol 30. Every outcome matched the
fixture. The fixtures record what the network did at protocol 29. This run asks
whether the same cases behave the same way at protocol 30, and they did. It is
a new observation made with a fresh set of throwaway accounts, and it sits beside
[`../testnet-2026-10-13`](../testnet-2026-10-13/README.md) and
[`../replay-2026-10-19`](../replay-2026-10-19/README.md), which stay unchanged.

## What was run

- Network: Stellar Testnet, protocol 30, `stellar-core 30.0.0 (0e49751aa9ddf892302cca02cea7f3f5d97df9f1)`.
- The switch: Testnet's last protocol 29 ledger is 5540312 (closed 2026-11-03T17:00:00Z)
  and its first protocol 30 ledger is 5540313 (closed 2026-11-03T17:00:05Z), read
  from Horizon's ledger records. Every ledger in this run is later than that.
- Ledgers: 5550429 (start) to 5550456 (last accepted transaction).
- Client: `@stellar/stellar-sdk` 17.3.0, run as `node replay.mjs --network-protocol 30`
  by `tools/replay/replay.mjs` as committed in `b1975cd` (merged by pull request #24).
  The file was not changed between the run and that commit.
- Run window: 2026-11-04T07:02:52.558Z to 2026-11-04T07:05:22.184Z.
- Accounts: throwaway keys generated at run time and funded by Friendbot. Secret
  keys were held in memory only and appear nowhere in `results.json`. Nothing was
  submitted to mainnet.

The core binary is the same build (`0e49751`) as in the 2026-10-19 run, which ran
on a network still at protocol 29. The difference between the two runs is the
protocol the network was running.

## Files

| File | Content |
|---|---|
| `results.json` | The raw output of the run, unmodified. |
| `README.md` | This summary. |

## Results

Each fixture's account was first set to the fixture's snapshot (`setup`), read
back, and checked against it. Then the plan's transactions were submitted with the
signatures the plan names. FX-05c has no setup step.

| Fixture | Step | Account | Observed | Transaction hash |
|---|---|---|---|---|
| FX-01 | setup | `GDPKS3..3NBU` | accepted, ledger 5550432 | `84688db41139c2ad6594666235019b4138092537bbca84bea7fbd1f21e677c69` |
| FX-01 | raise-high | `GDPKS3..3NBU` | accepted, ledger 5550433 | `32298aa37469220e3a2ffe9aeb9d9a3a9b5dfce4e1cf45ea12fd35f048e1ab63` |
| FX-01 | follow-up post-master-alone | `GDPKS3..3NBU` | rejected at submission: tx_failed / op_bad_auth | `4f131d5ee1f25967aa029b89ad77b82f0dd4dc07443f1f8cf502853a98100804` (computed, not in a ledger) |
| FX-02 | setup | `GADD2X..GXBY` | accepted, ledger 5550437 | `3a401d435c8e313b28c34bed3fe6b7c0f8ec92f039ed06f8b3486cc622b6e0f8` |
| FX-02 | rotate | `GADD2X..GXBY` | accepted, ledger 5550438 | `28001364dec40e36049058a4ceb198c8435b69a8506b41180375314e3b0e3177` |
| FX-03 | setup | `GDUOBG..ZOX4` | accepted, ledger 5550441 | `bc58f8aca3a7d3b61c5121641152fcc85fa6a11b720210bcc443c32b4d68259b` |
| FX-03 | zero-master | `GDUOBG..ZOX4` | accepted, ledger 5550442 | `a2a2d406ffc6fbd85df84b277c10fdbfa1f512eb274f3b279ed0f903f7dfcff5` |
| FX-05a | setup | `GAULTT..R6DX` | accepted, ledger 5550445 | `dee2636c1e98931baa4fddcc0ec29147d786898e35861b11fb9a04c1bae808e5` |
| FX-05a | one | `GAULTT..R6DX` | accepted, ledger 5550446 | `176fb1f8a865c37c0af0c1bd106d55c7e55e9e25628aab9bcc3a8e17bb9c6ee8` |
| FX-05b | setup | `GAZIWB..NXEY` | accepted, ledger 5550449 | `74cf151adff41067786a3b4424fdb8077f9913202047ced1e7d55de4bdfe1a6d` |
| FX-05b | first | `GAZIWB..NXEY` | accepted, ledger 5550450 | `c30e2d94abd10aa1a95cc0fb45e3a377863411387a8a85ad405140592447a869` |
| FX-05b | second | `GAZIWB..NXEY` | rejected: tx_failed / op_bad_auth | `18d4e7bc8d4c1616d1f2136defb6cca219b0bc7a5e0af4b94db1356189a586dd` (computed, not in a ledger) |
| FX-05c | add-b | `GD5UTQ..CFDJ` | rejected: tx_bad_auth_extra | `b6c652db4c523a580428e000683d0fca3c02858c9a2199c9fb3ab253b6403040` (computed, not in a ledger) |
| FX-07 | setup | `GAV3U3..CHOO` | accepted, ledger 5550456 | `2feb65f8edf0f2ca5463c40707b793ea514215e6632276ad89839093392b2cbd` |
| FX-07 | probe | `GAV3U3..CHOO` | rejected: tx_bad_auth | `aa5684670279b6a5eb3cbb517e6932377228fa08c3e705119cb061d1633c0aa4` (computed, not in a ledger) |

What each fixture expected, and so what matched:

- FX-01, FX-02, FX-03, FX-05a: the plan's one transaction is accepted.
- FX-05b: the first transaction is accepted and the second is rejected. FX-05a and
  FX-05b make the same two changes, as one transaction and as two.
- FX-05c: the transaction is rejected for an unused signature.
- FX-07: a change-nothing transaction signed by every signer is rejected, because
  the account's LOW threshold is above its total weight. This row is a probe whose
  expectation comes from the stated rule, not from `expected.json`.
- FX-01 follow-up: after the plan locked the account, the master key alone could
  not change the thresholds. Horizon refused it at submission. The account's
  sequence number and its signers and thresholds were identical before and after.

## Notes

- **Rejected transactions are not in any ledger.** They consumed no fee and no
  sequence number, and their hashes are computed, not looked up.
- **The follow-up is bounded evidence.** The tool searched Horizon's history for
  the follow-up's hash 5 times and did not find it. The tool itself
  records that this is not absolute proof of non-inclusion.
- **The tool records every refusal as `rejected`.** It does not tell a rejection
  at submission from a failure to apply, except in the follow-up path. These
  fixtures expect only `accepted` and `rejected`.

## What this does not show

- Only the seven fixtures and the one follow-up attempt (FX-01, `post-master-alone`)
  were run. Other inputs, and the other follow-up attempts of the 2026-10-13
  evidence, were not.
- It is a Testnet observation. Mainnet was still on protocol 29 when this was run.
- It does not change what SignerReach supports. At the time of this run the
  checker accepts snapshots recorded at protocol 29 only, and reports protocol 30
  as `INCONCLUSIVE`. Whether and how to change that is a separate decision.
