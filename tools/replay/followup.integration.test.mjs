import test from "node:test";
import assert from "node:assert/strict";
import { Keypair, xdr } from "@stellar/stellar-sdk";

import { replayExitCode, runFollowupAttempts } from "./replay.mjs";

function accountState(master, additional, sequence = "10", high = 2) {
  return {
    sequenceNumber: () => sequence,
    thresholds: { low_threshold: 1, med_threshold: 1, high_threshold: high },
    signers: [
      { key: master, weight: 1 },
      { key: additional, weight: 1 },
    ],
  };
}

function resultRecord(hash, { successful = true, ledger = 900, opResult } = {}) {
  const result = successful
    ? xdr.TransactionResultResult.txSuccess([xdr.OperationResult.opInner(
      xdr.OperationResultTr.setOptions(xdr.SetOptionsResult.setOptionsSuccess()),
    )])
    : xdr.TransactionResultResult.txFailed([xdr.OperationResult.opBadAuth()]);
  return {
    hash,
    successful,
    ledger_attr: ledger,
    result_xdr: new xdr.TransactionResult({
      feeCharged: 100n,
      result: opResult ?? result,
      ext: xdr.TransactionResultExt.v0(),
    }).toXdr("base64"),
  };
}

function horizonError(status, data = { title: "Horizon error" }, code) {
  const error = new Error("mock Horizon error");
  error.status = status;
  error.code = code;
  error.response = { status, data };
  return error;
}

function harness({ loadAccount, submitTransaction, history } = {}) {
  const master = Keypair.random();
  const additional = Keypair.random();
  const account = master;
  const keys = new Map([["master", master], ["additional", additional]]);
  const current = { value: accountState(master.publicKey(), additional.publicKey()) };
  const calls = { account: 0, submit: 0, history: 0, submittedHash: null };
  const server = {
    async loadAccount() {
      calls.account += 1;
      return loadAccount ? loadAccount(calls.account, current.value) : current.value;
    },
    async submitTransaction(transaction) {
      calls.submit += 1;
      calls.submittedHash = Buffer.from(transaction.hash()).toString("hex");
      return submitTransaction
        ? submitTransaction(transaction, calls, current)
        : { hash: calls.submittedHash, ledger: 900 };
    },
    transactions() {
      return { transaction: (hash) => ({ call: async () => {
        calls.history += 1;
        return history ? history(hash, calls, current) : resultRecord(hash);
      } }) };
    },
  };
  const attempt = (id = "one", expected = {
    stage: "accepted",
    result_codes: { transaction: "tx_success", operations: ["op_success"] },
  }) => ({
    id,
    signed_by: ["master"],
    operations: [{ op: "set_options", high: 0 }],
    expected,
  });
  return { master, additional, account, keys, current, calls, server, attempt };
}

const noWait = { sleep: async () => {}, historyChecks: 3, historyCheckDelayMs: 0 };

test("confirms successful inclusion after delayed history visibility", async () => {
  const h = harness({
    submitTransaction: async (_tx, calls, current) => {
      current.value = accountState(h.master.publicKey(), h.additional.publicKey(), "11", 0);
      return { hash: calls.submittedHash, ledger: 900 };
    },
    history: async (hash, calls) => {
      if (calls.history < 3) throw horizonError(404);
      return resultRecord(hash);
    },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(result.status, "accepted");
  assert.equal(result.match, true);
  assert.equal(result.hash, h.calls.submittedHash);
  assert.equal(result.ledger, 900);
  assert.equal(h.calls.history, 3);
});

test("classifies exact submission rejection codes only after bounded non-inclusion and unchanged state", async () => {
  const codes = { transaction: "tx_failed", operations: ["op_bad_auth"] };
  const h = harness({
    submitTransaction: async () => { throw horizonError(400, { extras: { result_codes: codes } }); },
    history: async () => { throw horizonError(404); },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt("reject", {
    stage: "rejected_at_submission", result_codes: codes,
  })], h.keys, noWait);
  assert.equal(result.status, "rejected_at_submission");
  assert.deepEqual(result.result_codes, codes);
  assert.equal(result.match, true);
  assert.equal(result.non_inclusion_checks, 3);
  assert.equal(result.non_inclusion_is_absolute_proof, false);
});

test("classifies an included failed transaction separately from submission rejection", async () => {
  const h = harness({
    submitTransaction: async () => { throw horizonError(400, { extras: { result_codes: {
      transaction: "tx_failed", operations: ["op_bad_auth"],
    } } }); },
    history: async (hash) => resultRecord(hash, { successful: false }),
    loadAccount: async (call, current) => {
      if (call > 1) current.sequenceNumber = () => "11";
      return current;
    },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt("failed", {
    stage: "failed_to_apply", result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
  })], h.keys, noWait);
  assert.equal(result.status, "failed_to_apply");
  assert.equal(result.match, true);
  assert.equal(result.ledger, 900);
  assert.equal(result.sequence_after, "11");
});

test("leaves a submission timeout unconfirmed when history cannot establish inclusion", async () => {
  const h = harness({
    submitTransaction: async () => { throw horizonError(504, { title: "Timed out" }, "ETIMEDOUT"); },
    history: async () => { throw horizonError(404); },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(result.status, "unconfirmed");
  assert.equal(result.match, null);
  assert.ok(result.hash);
  assert.deepEqual(result.raw_response.submission.response, { title: "Timed out" });
  assert.equal(Object.hasOwn(result, "ledger"), false);
});

test("does not treat an HTTP 503 body containing result codes as a definite rejection", async () => {
  const h = harness({
    submitTransaction: async () => { throw horizonError(503, { extras: { result_codes: {
      transaction: "tx_failed", operations: ["op_bad_auth"],
    } } }); },
    history: async () => { throw horizonError(404); },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt("maybe-rejected", {
    stage: "rejected_at_submission", result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
  })], h.keys, noWait);
  assert.equal(result.status, "unconfirmed");
  assert.equal(result.match, null);
});

test("retains history failure evidence and marks later attempts not_run", async () => {
  const h = harness({
    history: async () => { throw horizonError(503, { title: "History unavailable" }); },
  });
  const [first, second] = await runFollowupAttempts(h.server, h.account, [h.attempt(), h.attempt("two")], h.keys, noWait);
  assert.equal(first.status, "unconfirmed");
  assert.equal(first.match, null);
  assert.deepEqual(first.raw_response.history_lookup.response, { title: "History unavailable" });
  assert.equal(second.status, "not_run");
  assert.equal(h.calls.submit, 1);
  assert.equal(replayExitCode([{ after: [first, second] }], []), 2);
  assert.equal(Object.hasOwn(first, "ledger"), false);
  assert.doesNotThrow(() => JSON.parse(JSON.stringify({ results: [first, second] })));
});

test("marks a malformed history body unconfirmed and preserves its bounded representation", async () => {
  const h = harness({ history: async () => null });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(result.status, "unconfirmed");
  assert.equal(result.reason, "malformed_history_response");
  assert.deepEqual(result.raw_response.history_lookup.response, { omitted: "non_object_response" });
  assert.equal(Object.hasOwn(result, "ledger"), false);
});

test("returns a structured unconfirmed result when the initial account read times out", async () => {
  const h = harness({ loadAccount: async () => { throw horizonError(504, { title: "Account timeout" }); } });
  await assert.doesNotReject(async () => {
    const results = await runFollowupAttempts(h.server, h.account, [h.attempt(), h.attempt("two")], h.keys, noWait);
    assert.equal(results[0].status, "unconfirmed");
    assert.equal(results[0].match, null);
    assert.deepEqual(results[0].raw_response.account_read.response, { title: "Account timeout" });
    assert.equal(results[1].status, "not_run");
    assert.equal(h.calls.submit, 0);
  });
});

test("returns structured unconfirmed evidence if post-submission account reconciliation times out", async () => {
  const h = harness({
    loadAccount: async (call, current) => {
      if (call > 1) throw horizonError(504, { title: "Reconciliation timeout" });
      return current;
    },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(result.status, "unconfirmed");
  assert.equal(result.match, null);
  assert.equal(result.hash, h.calls.submittedHash);
  assert.equal(result.ledger, 900);
  assert.deepEqual(result.raw_response.account_reconciliation.response, { title: "Reconciliation timeout" });
});

test("retains a transient account read failure even when later reconciliation succeeds", async () => {
  const h = harness({
    loadAccount: async (call, current) => {
      if (call === 2) throw horizonError(504, { title: "Transient timeout" });
      return current;
    },
    submitTransaction: async (_tx, calls, current) => {
      current.value = accountState(h.master.publicKey(), h.additional.publicKey(), "11", 0);
      return { hash: calls.submittedHash, ledger: 900 };
    },
  });
  const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(result.status, "accepted");
  assert.equal(result.match, true);
  assert.equal(result.raw_response.account_read_failures[0].evidence.status_code, 504);
});

test("treats malformed account and submission responses as unconfirmed", async (t) => {
  await t.test("account body", async () => {
    const h = harness({ loadAccount: async () => ({ signers: [], thresholds: {} }) });
    const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
    assert.equal(result.status, "unconfirmed");
    assert.equal(result.reason, "malformed_account_response");
    assert.equal(result.attempted, false);
    assert.equal(h.calls.submit, 0);
  });
  await t.test("submission body", async () => {
    const h = harness({ submitTransaction: async () => null });
    const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
    assert.equal(result.status, "unconfirmed");
    assert.equal(result.match, null);
    assert.ok(result.raw_response.submission);
  });
  await t.test("unavailable account", async () => {
    const h = harness({ loadAccount: async () => { throw horizonError(404, { title: "Account not found" }); } });
    const [result] = await runFollowupAttempts(h.server, h.account, [h.attempt()], h.keys, noWait);
    assert.equal(result.status, "unconfirmed");
    assert.equal(result.reason, "account_read_failed");
    assert.equal(result.raw_response.account_read.status_code, 404);
    assert.equal(h.calls.submit, 0);
  });
});

test("keeps inconsistent sequence and changed authorization state unconfirmed", async (t) => {
  for (const variant of ["sequence", "authorization"]) {
    await t.test(variant, async () => {
      const h = harness({
        loadAccount: async (call, current) => {
          if (call > 1) return variant === "sequence"
            ? accountState(h.master.publicKey(), h.additional.publicKey(), "99", 0)
            : accountState(h.master.publicKey(), h.additional.publicKey(), "11", 7);
          return current;
        },
      });
      const [result, later] = await runFollowupAttempts(h.server, h.account, [h.attempt(), h.attempt("two")], h.keys, noWait);
      assert.equal(result.status, "unconfirmed");
      assert.equal(result.match, null);
      assert.equal(later.status, "not_run");
      assert.equal(h.calls.submit, 1);
    });
  }
});

test("dry-run follow-up builds and signs without touching a network client", async () => {
  const h = harness();
  const results = await runFollowupAttempts(null, h.account, [h.attempt()], h.keys, noWait);
  assert.equal(results[0].status, "not_submitted");
  assert.equal(results[0].match, null);
  assert.equal(h.calls.account + h.calls.submit + h.calls.history, 0);
});

test("returns exit code 2 for incomplete setup, transaction, or follow-up outcomes", () => {
  assert.equal(replayExitCode([{ setup: { status: "unconfirmed" } }], []), 2);
  assert.equal(replayExitCode([{ transactions: [{ status: "not_run" }] }], []), 2);
  assert.equal(replayExitCode([{ after: [{ status: "unconfirmed" }, { status: "not_run" }] }], []), 2);
  assert.equal(replayExitCode([{ match: false }], ["FX-01"]), 1);
});
