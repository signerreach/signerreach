# Security review

Date: 2026-10-31. Scope: the three Rust crates, the Node replay harness,
repository CI and the published security policy. This is a maintainer-side
source and test review, not an independent audit or a guarantee of safety.

## Method and boundaries

I traced untrusted JSON from CLI files and Horizon responses through parsing,
authorization, reports and errors. I checked file and network I/O, protocol
gates, size and time limits, duplicate keys, integer bounds, output fields,
secret handling and CI permissions. I used the fixture and fake-server tests,
the RustSec advisory scanner and npm audit. A scanner can identify known
dependency advisories; it does not prove the application secure.

| Finding | Evidence and correction | Current status |
|---|---|---|
| An unknown JSON field name and a CLI file name could be repeated in errors | Regression tests in `input.rs` and CLI `tests/check.rs` failed before [output sanitization](https://github.com/Hollujay/signerreach/commit/f271647). Unknown fields now report their object and entry index; file read errors name only the document role. | Corrected and tested |
| A Horizon response for a different account, or with contradictory IDs, could produce a snapshot | Fake-server regressions failed before [account consistency checks](https://github.com/Hollujay/signerreach/commit/9866a43). The adapter now checks requested `id`, `account_id`, duplicate JSON keys, and `last_modified_ledger` against the account response header. | Corrected and tested |
| An account flag change could receive a safe verdict without existing flag state | The regression failed before [the inconclusive guard](https://github.com/Hollujay/signerreach/commit/e0c9ae7). The official [Set Options result codes](https://developers.stellar.org/docs/data/apis/horizon/api-reference/errors/result-codes/operation-specific/set-options) include flag-dependent failures. | Corrected and tested |
| The Rust TLS lockfile contained two known trust-chain advisories | `cargo audit` found [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) and [RUSTSEC-2026-0381](https://rustsec.org/advisories/RUSTSEC-2026-0381.html). [The lockfile update](https://github.com/Hollujay/signerreach/commit/c85d1c8) raised only the affected packages. The subsequent scan reported no known vulnerabilities. | Corrected in lockfile; scanner clear on 2026-10-31 |
| The replay lockfile contained a high severity redirect advisory and an SDK predating Protocol 30 XDR | `npm audit --omit=dev` found [GHSA-mrwp-f3q8-3g9p](https://github.com/advisories/GHSA-mrwp-f3q8-3g9p). [The replay update](https://github.com/Hollujay/signerreach/commit/201c318) advanced `follow-redirects` and pinned the first [Protocol 30 SDK release](https://github.com/stellar/js-stellar-sdk/releases/tag/v17.3.0). The next audit reported zero vulnerabilities. Live replay is still restricted to protocol 29. | Corrected in lockfile; scanner clear on 2026-10-31 |

## Controls observed

- Each CLI input is capped at 1 MiB and parsed with duplicate-key detection.
  The JSON parser has a recursion limit. Plan and scenario counts, operation
  counts, signer counts, weights and thresholds have explicit bounds. Rust
  integer conversions reject out-of-range values.
- Validated public G addresses, transaction IDs, scenario names and home
  domains are intentionally visible in reports. IDs and names allow only
  restricted ASCII characters; home domains are limited to printable ASCII.
  Raw unknown names and malformed values are not echoed. JSON output escapes
  strings. Report ordering comes from ordered maps and fixed field ordering.
- `check` uses local files only. `fetch` uses only GET requests to a validated
  Horizon root and account path. It refuses redirects and proxies, uses no
  credential, limits each body to 1 MiB, and has connection and total timeouts.
  Plain HTTP is accepted only for loopback hostnames or addresses. Fake-server
  tests exercise the network and response boundary.
- The shipped Rust binary needs no signing key. Replay generates temporary
  Testnet keypairs in memory. Source inspection found no secret-key logging or
  persistence path. A Linux `strace -f -e trace=network` run of the dry run
  recorded no network system calls; all seven chain fixtures built and signed
  with `not_submitted` results. Live replay was not run in this review.
- CI uses `contents: read`, checkout without persisted credentials, pinned
  official actions, and pull-request jobs without repository secrets or live
  Testnet transactions. GitHub's private vulnerability reporting API returned
  `enabled: true` on 2026-10-31, matching [SECURITY.md](../SECURITY.md).

## Outstanding limitations

The checker cannot verify key custody, actual signature bytes, reserve
sufficiency, a fresh live state at submission time, or all possible behavior
outside its modeled `SetOptions` subset. A signer addition can fail for low
reserve despite a safe authorization and reachability result. Horizon's
root/account/root sequence checks network and protocol consistency, but is
not an atomic ledger snapshot and cannot defeat a stale or dishonest server.
Protocol 30 has not been verified for this checker. The historic Testnet runs
were on protocol 29. The replay harness does not repeat the original follow-up
lockout attempts. These are disclosed scope limits and must be considered when
using `SAFE WITHIN SCOPE`.

The Node development harness trusts repository fixtures and does not impose
the CLI's 1 MiB input cap. A user who places a secret in a field designed for
public display can expose it in a report or replay result. No user interviews,
independent security audit, live Protocol 30 test or current live Testnet replay
was performed here.
