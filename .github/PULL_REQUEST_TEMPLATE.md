## What changed and why

<!-- One purpose per pull request. Link the issue, for example "Closes #12". -->

## How you checked it

<!-- The commands you ran and what you saw. Say what you did not check. -->

## Checklist

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass
- [ ] If the change affects what the tool says: `docs/SPEC.md` is updated, and a fixture was written by hand from the spec
- [ ] Anything the tool does not model still gives `INCONCLUSIVE`
- [ ] No secret, key or signature appears in code, tests, fixtures or output
- [ ] No claim I cannot back (audited, adopted, approved, funded), and no synthetic test described as network evidence
- [ ] I understand and have run what I am submitting
