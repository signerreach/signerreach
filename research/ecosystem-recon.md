# Stellar ecosystem reconnaissance

Checked 2026-10-31. This is a source record, not a claim of exhaustive market
coverage or program eligibility.

## Mechanism and official sources

SignerReach analyzes the signer weights and LOW, MEDIUM and HIGH thresholds of
one Stellar classic account. Its immediate use case is a planned `SetOptions`
change that might leave HIGH unreachable. Stellar matters because its actual
transaction authorization order, signer types, limits and result codes decide
whether a plan works. A final-state weight sum alone cannot establish that.

| Source checked | What it supports | Date checked |
|---|---|---|
| [Stellar Core `SetOptionsOpFrame.cpp`](https://github.com/stellar/stellar-core/blob/master/src/transactions/SetOptionsOpFrame.cpp) | HIGH for administrative `SetOptions`; flag and reserve failure paths | 2026-10-31 |
| [Stellar Core `TransactionFrame.cpp`](https://github.com/stellar/stellar-core/blob/master/src/transactions/TransactionFrame.cpp) | LOW envelope check and transaction signature processing | 2026-10-31 |
| [CAP-0002](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0002.md) | Operations' signatures are checked before transaction side effects | 2026-10-31 |
| [Horizon account resource](https://developers.stellar.org/docs/data/apis/horizon/api-reference/resources/accounts) | Account reads and the `Latest-Ledger` response header | 2026-10-31 |
| [Set Options result codes](https://developers.stellar.org/docs/data/apis/horizon/api-reference/errors/result-codes/operation-specific/set-options) | Low reserve, flag, signer and home-domain failure classes | 2026-10-31 |
| [Bell, Protocol 30 upgrade guide](https://stellar.org/blog/developers/bell-protocol-30-upgrade-guide) | Testnet and Mainnet vote dates and required SDK upgrade | 2026-10-31 |

The repository's [original Testnet observations](../docs/evidence/testnet-2026-10-13/README.md)
and [second replay](../docs/evidence/replay-2026-10-19/README.md) concern
protocol 29. The planned Protocol 30 votes are November 3 on Testnet and
November 23 on Mainnet, both at 17:00 UTC, according to the official guide
checked above. Its published changes concern muxed contract addresses, smart
contract signature verification and ledger time representation. That list
does not prove the classic-account rules unchanged. The checker continues to
reject other protocol versions as inconclusive. The replay SDK was advanced to
[v17.3.0](https://github.com/stellar/js-stellar-sdk/releases/tag/v17.3.0),
which contains Protocol 30 XDR, but live replay remains guarded at protocol 29.

## Adjacent work

The [README comparison](../README.md#how-it-differs-from-other-tools) records a
source inspection from 2026-10-13 and a recheck on 2026-10-20. It identified
Stellar Laboratory, [Stellar Stratum](https://github.com/tupui/stellar-stratum),
[stellar-toml-lint](https://github.com/anchor-tools/stellar-toml-lint) and a
[JavaScript SDK proposal](https://github.com/stellar/js-stellar-sdk/issues/1606)
as adjacent tools or discussions. This review did not re-audit every current
feature of those products, nor any closed-source wallet. The defensible
distinction in this repository is its implemented combination of ordered
transaction plans, declared signer availability scenarios, an offline report,
and chain-backed fixtures for the authorization-order cases. It does not claim
exclusive ownership of these ideas.

## Program status and open questions

The [Stellar Community Fund Build Award page](https://communityfund.stellar.org/awards)
lists Open, Integration and RFP tracks and directs applicants to track-specific
criteria and official rules. It publicly lists a current submission route.
SignerReach's eligibility for a track, invitation or award is **unverified**.
There is no evidence here of user interviews, product-market fit, a budget,
funding or approval. The maintainer must check the current official rules and
choose a track before submitting.

Open questions: whether a maintainer wants independent Protocol 30 verification,
whether users can reliably supply accurate availability declarations, whether
the three existing contributor issues are priorities for a first release, and
how issue #3 should represent follow-up attempts. None is answered by the
historical Testnet results.
