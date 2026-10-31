# SignerReach documentation

SignerReach is an unreleased Rust CLI for reviewing planned signer and
threshold changes to one Stellar classic account. The current verified rules
boundary is protocol 29. Start with the [repository README](../README.md) for
the purpose, build command and short example.

| Need | Read |
|---|---|
| Problem and intended users | [Problem validation record](../research/problem-validation.md) |
| Why Stellar and how the tool differs | [Ecosystem research](../research/ecosystem-recon.md) |
| Architecture and trust boundaries | [Architecture](architecture.md) |
| Supported protocol, networks and exclusions | [Specification, scope](SPEC.md#3-scope) and [architecture limits](architecture.md#future-work-and-explicit-exclusions) |
| Install and run `check` or `fetch` | [README, building](../README.md#building), [making a snapshot](../README.md#making-a-snapshot), [command reference](SPEC.md#9-command-line) |
| Snapshot, plan and availability formats | [Specification, inputs](SPEC.md#5-inputs) and [fixtures](../fixtures/README.md) |
| Authorization and transaction pre-state semantics | [Specification, verified rules](SPEC.md#4-verified-rules-this-specification-relies-on) and [evaluation](SPEC.md#6-evaluation) |
| Verdicts, diagnostics and exit codes | [Specification, diagnostics](SPEC.md#7-diagnostic-codes) and [README exit codes](../README.md#exit-codes) |
| Unsafe and corrected examples | [Walkthrough](walkthrough/README.md) |
| Fixtures and Testnet evidence | [Evidence index](../evidence/index.md), [original Testnet run](evidence/testnet-2026-10-13/README.md), [second replay](evidence/replay-2026-10-19/README.md) |
| Security model and known limitations | [Security policy](../SECURITY.md), [security review](security-review.md), [architecture](architecture.md#inputs-and-trust-boundaries) |
| Contributor setup and testing | [Contributing guide](../CONTRIBUTING.md), [fixture guide](../fixtures/README.md) |
| Reproduce the demonstration | [Walkthrough commands](walkthrough/README.md#2-check-the-plan), [replay instructions](../tools/replay/README.md) |
| Repository review status | [Readiness audit](readiness-audit.md) |

The Testnet records are historical protocol 29 evidence. The second replay
does not repeat the original follow-up lockout attempts. No site URL or hosted
application is needed to use this CLI. The repository documentation is the
maintained review surface.
