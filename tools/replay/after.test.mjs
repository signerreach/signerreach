import test from "node:test";
import assert from "node:assert/strict";
import { StrKey, xdr } from "@stellar/stellar-sdk";

import {
  AFTER_SCHEMA,
  classifyAttemptOutcome,
  classifyLedgerRecord,
  compareResultCodes,
  parseAfterJson,
  sanitizeHorizonResponse,
  validateAfterDocument,
} from "./after.mjs";

const account = "GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQDZ7H";
const other = "GABAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEJXA";

function document() {
  return {
    schema: AFTER_SCHEMA,
    attempts: [{
      id: "post-master-alone",
      signed_by: [account],
      operations: [{ op: "set_options", high: 0 }],
      expected: {
        stage: "rejected_at_submission",
        result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
      },
    }],
  };
}

function record({ successful, result }) {
  const resultXdr = new xdr.TransactionResult({
    feeCharged: 100n,
    result,
    ext: xdr.TransactionResultExt.v0(),
  }).toXdr("base64");
  return { successful, result_xdr: resultXdr };
}

test("accepts an explicit available master signer and a rejection expectation", () => {
  const errors = validateAfterDocument(document(), {
    knownAddresses: new Set([account, other]),
    isAvailable: (address) => address === account,
  });
  assert.deepEqual(errors, []);
});

test("requires exact expected transaction and ordered operation codes for accepted stages", () => {
  const value = document();
  value.attempts[0].expected = {
    stage: "accepted",
    result_codes: { transaction: "tx_success", operations: ["op_success"] },
  };
  assert.deepEqual(validateAfterDocument(value, {
    knownAddresses: new Set([account]),
    isAvailable: () => true,
  }), []);
});

test("rejects unavailable, unknown and duplicate signers", () => {
  const value = document();
  value.attempts[0].signed_by = [account, other, other];
  const errors = validateAfterDocument(value, {
    knownAddresses: new Set([account, other]),
    isAvailable: (address) => address === account,
  });
  assert.equal(errors.length, 2);
  assert.match(errors.join("\n"), /not declared available/);
  assert.match(errors.join("\n"), /duplicate signer/);
});

test("rejects contract and non-Ed25519 signer address types", () => {
  const contract = StrKey.encodeContract(Buffer.alloc(32));
  const hashX = StrKey.encodeSha256Hash(Buffer.alloc(32));
  const value = document();
  value.attempts[0].signed_by = [contract];
  value.attempts[0].operations[0].signer = { key: hashX, weight: 1 };
  const errors = validateAfterDocument(value, {
    knownAddresses: new Set([contract, hashX]),
    isAvailable: () => true,
  });
  assert.match(errors.join("\n"), /known ed25519 G address/);
});

test("rejects unsupported fields, stages and duplicate attempt ids", () => {
  const value = document();
  value.attempts.push({ ...value.attempts[0], operations: [{ op: "payment" }] });
  value.attempts[0].unexpected = true;
  const errors = validateAfterDocument(value, {
    knownAddresses: new Set([account]),
    isAvailable: () => true,
  });
  assert.match(errors.join("\n"), /unsupported field/);
  assert.match(errors.join("\n"), /duplicate attempt id/);
  assert.match(errors.join("\n"), /only set_options/);
});

test("malformed JSON is rejected before any replay action", () => {
  assert.deepEqual(parseAfterJson("{", { knownAddresses: new Set(), isAvailable: () => false }), {
    document: null,
    errors: ["$: invalid JSON"],
  });
});

test("duplicate JSON keys are rejected before values are interpreted", () => {
  const duplicate = `{"schema":"${AFTER_SCHEMA}","attempts":[],"attempts":[]}`;
  assert.deepEqual(parseAfterJson(duplicate, { knownAddresses: new Set(), isAvailable: () => false }), {
    document: null,
    errors: ["$: duplicate object key"],
  });
});

test("classifies transaction failure from ledger XDR separately from submission rejection", () => {
  const failed = record({
    successful: false,
    result: xdr.TransactionResultResult.txFailed([xdr.OperationResult.opBadAuth()]),
  });
  assert.deepEqual(classifyLedgerRecord(failed), {
    status: "failed_to_apply",
    result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
  });
  assert.equal(classifyLedgerRecord(null).status, "unconfirmed");
});

test("classifies bounded submission rejection only when account sequence and state are unchanged", () => {
  const codes = { transaction: "tx_failed", operations: ["op_bad_auth"] };
  assert.deepEqual(classifyAttemptOutcome({
    historyAbsent: true,
    submissionResultCodes: codes,
    accountUnchanged: true,
  }), {
    status: "rejected_at_submission",
    result_codes: codes,
    non_inclusion_is_absolute_proof: false,
  });
  assert.deepEqual(classifyAttemptOutcome({
    historyAbsent: true,
    submissionResultCodes: codes,
    accountUnchanged: false,
  }), { status: "unconfirmed", reason: "account_changed_after_submission_rejection" });
});

test("classifies successful included ledger result", () => {
  const succeeded = record({
    successful: true,
    result: xdr.TransactionResultResult.txSuccess([xdr.OperationResult.opInner(
      xdr.OperationResultTr.setOptions(xdr.SetOptionsResult.setOptionsSuccess()),
    )]),
  });
  assert.deepEqual(classifyLedgerRecord(succeeded), {
    status: "accepted",
    result_codes: { transaction: "tx_success", operations: ["op_success"] },
  });
});

test("compares ordered structured result codes exactly", () => {
  const actual = { transaction: "tx_failed", operations: ["op_bad_auth", "op_no_account"] };
  assert.equal(compareResultCodes(actual, actual), true);
  assert.equal(compareResultCodes(actual, { ...actual, operations: [...actual.operations].reverse() }), false);
  assert.equal(compareResultCodes(actual, { ...actual, operations: ["op_bad_auth"] }), false);
});

test("stores only bounded Horizon response fields and omits sensitive payloads", () => {
  const saved = sanitizeHorizonResponse({
    title: "Transaction Failed",
    extras: {
      result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
      result_xdr: "YWJj",
      envelope_xdr: "DO_NOT_STORE",
    },
    envelope_xdr: "DO_NOT_STORE",
    debug: "DO_NOT_STORE",
  });
  assert.deepEqual(saved, {
    title: "Transaction Failed",
    extras: {
      result_codes: { transaction: "tx_failed", operations: ["op_bad_auth"] },
      result_xdr: "YWJj",
    },
  });
  const large = sanitizeHorizonResponse({ title: "x".repeat(25 * 1024) });
  assert.deepEqual(large.title, { omitted: "field_over_limit", bytes: 25 * 1024 });
});
