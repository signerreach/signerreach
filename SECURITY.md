# Security policy

SignerReach tells people whether a change to a Stellar account is safe to make.
A wrong "safe" answer can cost someone their account, so correctness problems
are treated as security problems here.

## What to report privately

- **A false safe result.** The tool reports `SAFE WITHIN SCOPE` for a case where,
  under the stated assumptions, administrative access would be lost or the plan
  would be rejected by the network. Please include the evidence that shows
  Stellar behaves differently from the tool, such as a Testnet transaction hash.
- **Anything that handles a secret.** The tool never needs a private key, seed or
  signature. Any path that reads, stores, logs or prints one is a bug, as is any
  message that repeats a value taken from an input document.
- **A crash or unbounded use of memory or time on crafted input.** All input is
  untrusted, and so is everything a server sends to `fetch`.
- **A `fetch` request that goes somewhere it should not.** It must send only `GET`
  requests for the server root and one account, follow no redirect, use no proxy,
  and send no credential. Anything else is a bug.
- **A weakness in the build or release process**, such as a workflow that could
  be made to run with more permissions than it needs.

## What to report in the open

Open an ordinary issue for everything else: unclear wording, a missing feature,
or a result that is too cautious (for example `INCONCLUSIVE` or `UNSAFE` where
`SAFE WITHIN SCOPE` would be correct). Being cautious is by design, but such
cases are still worth fixing.

## How to report

Use GitHub's private reporting: open the **Security** tab of this repository and
choose **Report a vulnerability**. Please do not open a public issue or pull
request for something described above.

Include:

- the three input files (`snapshot.json`, `plan.json`, `availability.json`) or a
  description that lets someone recreate them;
- the output you got and the output you expected;
- the version or commit you ran.

**Never send a private key, a seed or a signature.** None of them is needed.

## What to expect

- An acknowledgement within 7 days.
- An assessment, and updates when something changes.
- A fix on a best-effort basis. This is an unfunded open-source project and no
  fixed repair time is promised.
- Credit in the advisory, unless you prefer not to be named.
- No payment: there is no bounty programme.

When a fix is released, a security advisory describing the problem is published.

## Supported versions

There are no releases yet. Only the `main` branch is supported.

## What this policy does not promise

SignerReach has not been audited, and its results depend on the availability
you declare, which it cannot verify. See the coverage section of any report, and
the warranty disclaimer in the [license](LICENSE).
