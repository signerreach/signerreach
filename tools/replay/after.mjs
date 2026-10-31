import { StrKey, xdr } from "@stellar/stellar-sdk";

export const AFTER_SCHEMA = "signerreach.replay-after/1";
export const HISTORY_CHECKS = 5;
export const HISTORY_CHECK_DELAY_MS = 1000;
export const MAX_RAW_FIELD_BYTES = 24 * 1024;

const stages = new Set(["accepted", "rejected_at_submission", "failed_to_apply"]);
const idPattern = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;
const codePattern = /^[a-z][a-z0-9_]*$/;

export function validateAfterDocument(document, { knownAddresses, isAvailable }) {
  const errors = [];
  const issue = (path, message) => errors.push(`${path}: ${message}`);

  if (!isObject(document)) return ["$: expected an object"];
  for (const key of Object.keys(document)) {
    if (!["schema", "attempts"].includes(key)) issue(`/${key}`, "unsupported field");
  }
  if (document.schema !== AFTER_SCHEMA) issue("/schema", `expected ${AFTER_SCHEMA}`);
  if (!Array.isArray(document.attempts)) {
    issue("/attempts", "expected an array");
    return errors;
  }
  if (document.attempts.length > 100) issue("/attempts", "at most 100 attempts are supported");

  const ids = new Set();
  for (const [index, attempt] of document.attempts.entries()) {
    const path = `/attempts/${index}`;
    if (!isObject(attempt)) {
      issue(path, "expected an object");
      continue;
    }
    for (const key of Object.keys(attempt)) {
      if (!["id", "signed_by", "operations", "expected"].includes(key)) issue(`${path}/${key}`, "unsupported field");
    }
    if (typeof attempt.id !== "string" || !idPattern.test(attempt.id)) {
      issue(`${path}/id`, "expected a unique 1 to 64 character replay id");
    } else if (ids.has(attempt.id)) {
      issue(`${path}/id`, "duplicate attempt id");
    } else {
      ids.add(attempt.id);
    }
    if (Object.hasOwn(attempt, "source")) issue(`${path}/source`, "source overrides are not supported");

    if (!Array.isArray(attempt.signed_by) || attempt.signed_by.length === 0) {
      issue(`${path}/signed_by`, "expected at least one explicit signer");
    } else {
      const signers = new Set();
      for (const [signerIndex, address] of attempt.signed_by.entries()) {
        const signerPath = `${path}/signed_by/${signerIndex}`;
        if (typeof address !== "string" || !StrKey.isValidEd25519PublicKey(address) || !knownAddresses.has(address)) {
          issue(signerPath, "signer must be a known ed25519 G address defined by this fixture");
        } else if (signers.has(address)) {
          issue(signerPath, "duplicate signer");
        } else {
          signers.add(address);
          if (!isAvailable(address)) issue(signerPath, "signer is not declared available");
        }
      }
    }

    if (!Array.isArray(attempt.operations) || attempt.operations.length === 0) {
      issue(`${path}/operations`, "expected at least one operation");
    } else {
      for (const [opIndex, operation] of attempt.operations.entries()) {
        const opPath = `${path}/operations/${opIndex}`;
        if (!isObject(operation)) {
          issue(opPath, "expected an object");
          continue;
        }
        if (Object.hasOwn(operation, "source")) issue(`${opPath}/source`, "operation sources are not supported");
        if (operation.op !== "set_options") issue(`${opPath}/op`, "only set_options is supported");
        for (const field of ["master_weight", "low", "medium", "high", "set_flags", "clear_flags"]) {
          if (Object.hasOwn(operation, field) && (!Number.isInteger(operation[field]) || operation[field] < 0 || operation[field] > 255)) {
            issue(`${opPath}/${field}`, "expected an integer from 0 to 255");
          }
        }
        if (Object.hasOwn(operation, "signer")) {
          if (!isObject(operation.signer)) {
            issue(`${opPath}/signer`, "expected a signer object");
          } else {
            if (typeof operation.signer.key !== "string" || !StrKey.isValidEd25519PublicKey(operation.signer.key) || !knownAddresses.has(operation.signer.key)) {
              issue(`${opPath}/signer/key`, "signer key must be a known ed25519 G address defined by this fixture");
            }
            if (!Number.isInteger(operation.signer.weight) || operation.signer.weight < 0 || operation.signer.weight > 255) {
              issue(`${opPath}/signer/weight`, "expected an integer from 0 to 255");
            }
          }
        }
        if (Object.hasOwn(operation, "home_domain") && (typeof operation.home_domain !== "string" || operation.home_domain.length > 32)) {
          issue(`${opPath}/home_domain`, "expected a string of at most 32 characters");
        }
        const allowed = new Set(["op", "master_weight", "low", "medium", "high", "signer", "home_domain", "set_flags", "clear_flags"]);
        for (const key of Object.keys(operation)) {
          if (!allowed.has(key)) issue(`${opPath}/${key}`, "unsupported set_options field");
        }
      }
    }

    if (!isObject(attempt.expected) || !stages.has(attempt.expected.stage)) {
      issue(`${path}/expected/stage`, "expected accepted, rejected_at_submission, or failed_to_apply");
    } else if (attempt.expected.stage === "accepted") {
      for (const key of Object.keys(attempt.expected)) {
        if (!["stage", "result_codes"].includes(key)) issue(`${path}/expected/${key}`, "unsupported field for accepted stage");
      }
      validateExpectedCodes(attempt.expected.result_codes, `${path}/expected/result_codes`, issue);
    } else {
      for (const key of Object.keys(attempt.expected)) {
        if (!["stage", "result_codes"].includes(key)) issue(`${path}/expected/${key}`, "unsupported field");
      }
      validateExpectedCodes(attempt.expected.result_codes, `${path}/expected/result_codes`, issue);
    }
  }
  return errors;
}

function validateExpectedCodes(codes, path, issue) {
  if (!isObject(codes) || typeof codes.transaction !== "string" || !codePattern.test(codes.transaction)) {
    issue(path, "expected a transaction result code");
  }
  if (!isObject(codes) || !Array.isArray(codes.operations) || !codes.operations.every((code) => typeof code === "string" && codePattern.test(code))) {
    issue(`${path}/operations`, "expected an ordered array of operation result codes");
  }
}

export function parseAfterJson(text, validation) {
  let document;
  try {
    document = JSON.parse(text);
  } catch {
    return { document: null, errors: ["$: invalid JSON"] };
  }
  if (hasDuplicateKeys(text)) return { document: null, errors: ["$: duplicate object key"] };
  return { document, errors: validateAfterDocument(document, validation) };
}

function hasDuplicateKeys(text) {
  let index = 0;
  let duplicate = false;
  const whitespace = () => {
    while (/\s/.test(text[index] ?? "")) index += 1;
  };
  const stringValue = () => {
    const start = index++;
    let escaped = false;
    while (index < text.length) {
      const character = text[index++];
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') return JSON.parse(text.slice(start, index));
    }
    return null;
  };
  const value = (depth = 0) => {
    if (depth > 128) {
      duplicate = true;
      return;
    }
    whitespace();
    if (text[index] === "{") {
      index += 1;
      whitespace();
      const keys = new Set();
      if (text[index] === "}") {
        index += 1;
        return;
      }
      while (index < text.length) {
        whitespace();
        const key = stringValue();
        if (keys.has(key)) duplicate = true;
        keys.add(key);
        whitespace();
        index += 1; // colon; JSON.parse above already verified syntax
        value(depth + 1);
        whitespace();
        const delimiter = text[index++];
        if (delimiter === "}") return;
      }
    } else if (text[index] === "[") {
      index += 1;
      whitespace();
      if (text[index] === "]") {
        index += 1;
        return;
      }
      while (index < text.length) {
        value(depth + 1);
        whitespace();
        const delimiter = text[index++];
        if (delimiter === "]") return;
      }
    } else if (text[index] === '"') {
      stringValue();
    } else {
      while (index < text.length && !/[\s,\]}]/.test(text[index])) index += 1;
    }
  };
  value();
  return duplicate;
}

export function resultCodesFromXdr(resultXdr) {
  const result = xdr.TransactionResult.fromXdr(resultXdr, "base64").result;
  const transaction = camelToSnake(result.type);
  const operations = result.results?.map((operation) => {
    if (operation.type !== "opInner") return camelToSnake(operation.type);
    const innerType = operation.tr.value?.type;
    if (!innerType) return "op_inner";
    const operationPrefix = operation.tr.type;
    const suffix = innerType.startsWith(operationPrefix)
      ? innerType.slice(operationPrefix.length)
      : innerType;
    return `op_${camelToSnake(suffix).replace(/^_/, "")}`;
  }) ?? [];
  return { transaction, operations };
}

export function classifyLedgerRecord(record) {
  if (!record || typeof record.successful !== "boolean" || typeof record.result_xdr !== "string") {
    return { status: "unconfirmed", reason: "incomplete_ledger_record" };
  }
  let resultCodes;
  try {
    resultCodes = resultCodesFromXdr(record.result_xdr);
  } catch {
    return { status: "unconfirmed", reason: "invalid_ledger_result" };
  }
  if (record.successful && resultCodes.transaction === "tx_success") {
    return { status: "accepted", result_codes: resultCodes };
  }
  if (!record.successful && resultCodes.transaction === "tx_failed") {
    return { status: "failed_to_apply", result_codes: resultCodes };
  }
  return { status: "unconfirmed", reason: "ledger_result_status_mismatch", result_codes: resultCodes };
}

export function classifyAttemptOutcome({ record, historyError, historyAbsent, submissionResultCodes, accountUnchanged }) {
  if (historyError) return { status: "unconfirmed", reason: "history_lookup_failed" };
  if (record) return classifyLedgerRecord(record);
  if (!historyAbsent) return { status: "unconfirmed", reason: "history_not_found_after_bounded_checks" };
  if (!isResultCodes(submissionResultCodes)) {
    return { status: "unconfirmed", reason: "submission_result_codes_missing_or_invalid" };
  }
  if (!accountUnchanged) {
    return { status: "unconfirmed", reason: "account_changed_after_submission_rejection" };
  }
  return {
    status: "rejected_at_submission",
    result_codes: submissionResultCodes,
    non_inclusion_is_absolute_proof: false,
  };
}

export function compareResultCodes(actual, expected) {
  return Boolean(
    actual && expected && actual.transaction === expected.transaction &&
    Array.isArray(actual.operations) && Array.isArray(expected.operations) &&
    actual.operations.length === expected.operations.length &&
    actual.operations.every((code, index) => code === expected.operations[index]),
  );
}

export function isResultCodes(value) {
  return Boolean(
    isObject(value) && typeof value.transaction === "string" && codePattern.test(value.transaction) &&
    Array.isArray(value.operations) && value.operations.every((code) => typeof code === "string" && codePattern.test(code)),
  );
}

export function sanitizeHorizonResponse(data) {
  if (!isObject(data)) return { omitted: "non_object_response" };
  const sanitized = {};
  for (const key of ["type", "title", "status", "hash", "successful", "ledger", "ledger_attr", "created_at", "fee_charged", "result_xdr"]) {
    if (["string", "number", "boolean"].includes(typeof data[key])) sanitized[key] = bounded(data[key]);
  }
  if (isObject(data.extras)) {
    const extras = {};
    if (isObject(data.extras.result_codes)) {
      extras.result_codes = {
        transaction: bounded(data.extras.result_codes.transaction),
        operations: Array.isArray(data.extras.result_codes.operations)
          ? data.extras.result_codes.operations.map(bounded)
          : [],
      };
    }
    if (typeof data.extras.result_xdr === "string") extras.result_xdr = bounded(data.extras.result_xdr);
    if (Object.keys(extras).length) sanitized.extras = extras;
  }
  const serialized = JSON.stringify(sanitized);
  if (Buffer.byteLength(serialized) <= MAX_RAW_FIELD_BYTES) return sanitized;
  return { omitted: "sanitized_response_over_limit", bytes: Buffer.byteLength(serialized) };
}

function bounded(value) {
  if (typeof value !== "string" || Buffer.byteLength(value) <= MAX_RAW_FIELD_BYTES) return value;
  return { omitted: "field_over_limit", bytes: Buffer.byteLength(value) };
}

function camelToSnake(value) {
  return value.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`).replace(/^_/, "");
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
