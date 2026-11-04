# Understanding the verdict

SignerReach checks **authorization** and **administrative reachability** within a deliberately narrow model. These are related but different questions.

## Authorization before application

A Stellar classic-account transaction has an envelope source and one or more operations. A signer must satisfy the relevant thresholds. The model includes LOW authorization of the envelope and HIGH authorization for administrative `SetOptions` changes.

A crucial distinction is that operations inside a single transaction are authorized against the account state **before that transaction**. Therefore, adding a signer and then changing a threshold inside one transaction can behave differently from doing those changes in separate transactions.

The historical [FX-05a and FX-05b cases](evidence.md) demonstrate that difference on Protocol 29 Testnet, and the same outcomes were observed on Protocol 30 Testnet.

## Administrative reachability

After applying the ordered plan, SignerReach assesses whether the signer keys declared available can still meet the administrative threshold. Availability is provided by the operator, not discovered or proved by the tool.

| Verdict | Meaning |
| --- | --- |
| **SAFE WITHIN SCOPE** | Every declared scenario has a way to administer the account under the modeled rules. |
| **UNSAFE** | The proposed plan is rejected or a declared scenario loses administrative access. |
| **INCONCLUSIVE** | The model lacks information or encounters an input or protocol feature it does not safely support. |

`SAFE WITHIN SCOPE` is not a promise that a transaction will execute successfully. Fees, reserves, state changes, key custody and other external conditions can matter. An account snapshot becomes stale as the ledger advances.

## Trace and coverage

The report shows signer weights, required thresholds, evaluated transactions, diagnostic codes and declared scenarios. It also states what was not modeled. For precise rules see the [specification](https://github.com/signerreach/signerreach/blob/main/docs/SPEC.md), and for an illustrated example see the [walkthrough](https://github.com/signerreach/signerreach/blob/main/docs/walkthrough/README.md).
