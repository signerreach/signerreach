# Contributing

SignerReach values reproducible evidence, small changes and accurate outcomes over feature volume. A false `SAFE WITHIN SCOPE` result is a serious correctness issue.

Start with the [contributing guide](https://github.com/signerreach/signerreach/blob/main/CONTRIBUTING.md), [specification](https://github.com/signerreach/signerreach/blob/main/docs/SPEC.md) and [fixture guide](https://github.com/signerreach/signerreach/blob/main/fixtures/README.md).

## Contributor opportunities

- [Issue #1](https://github.com/signerreach/signerreach/issues/1): add a JSON Schema inspection command.
- [Issue #2](https://github.com/signerreach/signerreach/issues/2): add a Markdown report renderer.
- [Issue #7](https://github.com/signerreach/signerreach/issues/7): investigate Protocol 30 with version-specific source review and new evidence; **do not enable a new protocol gate without approval**.

The initial follow-up fixture-format work under [issue #3](https://github.com/signerreach/signerreach/issues/3) is **closed** after implementation and FX-01 live evidence. Further scenarios require fresh scope approval.

## Working rules

Use one commit per genuine logical change, with tests. Keep reports deterministic, avoid introducing new dependencies without a need, never write secrets to evidence, and leave historical Testnet records intact. Ordinary pull-request CI must not submit live transactions.

Follow the [security policy](https://github.com/signerreach/signerreach/blob/main/SECURITY.md) rather than opening a public issue if you identify a false-safe result or secret-exposure vulnerability.
