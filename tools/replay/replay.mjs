// Replays the SignerReach "chain" fixtures on Stellar Testnet.
//
// For each fixture marked `chain`, it builds the fixture's account on Testnet
// with fresh throwaway keys, submits the plan's transactions with the
// signatures the plan names, and compares what the network did with the
// `transactions` list in the fixture's expected.json.
//
// This is a development tool. It signs and submits, which the shipped
// `signerreach` binary never does. Secret keys exist only in memory and are
// never written or printed. It refuses to run against anything but Testnet.
//
// Usage: node replay.mjs [--dry-run] [--only TEXT] [--out FILE]
//   --dry-run   build and sign every transaction, but touch no network
//   --only      run only fixtures whose directory name contains TEXT
//   --out       write the JSON results to FILE (default: standard output)
//
// Exit code: 0 every outcome matched, 1 an outcome differed, 2 the run could
// not be completed (including an unconfirmed follow-up), unsupported fixture,
// or bad usage.

import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  Account,
  BASE_FEE,
  Horizon,
  Keypair,
  Networks,
  Operation,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import {
  HISTORY_CHECK_DELAY_MS,
  HISTORY_CHECKS,
  classifyAttemptOutcome,
  compareResultCodes,
  isResultCodes,
  parseAfterBytes,
  parseAfterJson,
  sanitizeHorizonResponse,
} from "./after.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(HERE, "..", "..", "fixtures");
const HORIZON = "https://horizon-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const SDK_VERSION = JSON.parse(readFileSync(join(HERE, "package.json"), "utf8")).dependencies[
  "@stellar/stellar-sdk"
];

class Unusable extends Error {}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const log = (text) => process.stderr.write(`${text}\n`);
const short = (key) => `${key.slice(0, 5)}..${key.slice(-4)}`;
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));

function parseArgs(argv) {
  const options = { dryRun: false, only: null, out: null };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") options.dryRun = true;
    else if (arg === "--only" || arg === "--out") {
      i += 1;
      if (i >= argv.length) throw new Unusable(`${arg} needs a value`);
      options[arg === "--only" ? "only" : "out"] = argv[i];
    } else throw new Unusable(`unknown argument ${arg}`);
  }
  return options;
}

// ---------------------------------------------------------------- fixtures

function loadChainFixtures(only) {
  const fixtures = [];
  for (const name of readdirSync(FIXTURES).sort()) {
    if (!name.startsWith("fx-") || (only && !name.includes(only))) continue;
    const directory = join(FIXTURES, name);
    const expected = readJson(join(directory, "expected.json"));
    if (expected.source !== "chain") continue;
    const afterPath = join(directory, "after.json");
    fixtures.push({
      name,
      expected,
      snapshot: readJson(join(directory, "snapshot.json")),
      plan: readJson(join(directory, "plan.json")),
      availability: readJson(join(directory, "availability.json")),
      afterBytes: existsSync(afterPath) ? readFileSync(afterPath) : null,
    });
  }
  if (fixtures.length === 0) throw new Unusable("no chain fixtures matched");
  for (const fixture of fixtures) validateReplayAfter(fixture);
  return fixtures;
}

function validateReplayAfter(fixture) {
  const { snapshot, plan, availability, afterBytes, name } = fixture;
  const knownAddresses = new Set([snapshot.account]);
  for (const signer of snapshot.signers) knownAddresses.add(signer.key);
  for (const tx of plan.transactions) {
    tx.signed_by.forEach((address) => knownAddresses.add(address));
    for (const op of tx.operations) if (op.signer?.key) knownAddresses.add(op.signer.key);
  }
  const statusOf = (address) => availability.signers?.[address] ?? availability.default ?? "unknown";
  const parse = afterBytes ? parseAfterBytes : parseAfterJson;
  const text = afterBytes ?? JSON.stringify({ schema: "signerreach.replay-after/1", attempts: [] });
  const { document, errors } = parse(text, {
    knownAddresses,
    isAvailable: (address) => statusOf(address) === "available",
  });
  if (errors.length) throw new Unusable(`${name}: invalid after.json (${errors.join("; ")})`);
  fixture.after = document;
}

// One fresh keypair for every address the fixture mentions. The fixture's own
// account becomes the account that is built and tested.
function keypairsFor({ snapshot, plan, after }) {
  const keys = new Map();
  const add = (address) => {
    if (!keys.has(address)) keys.set(address, Keypair.random());
  };
  add(snapshot.account);
  for (const signer of snapshot.signers) add(signer.key);
  for (const tx of plan.transactions) {
    tx.signed_by.forEach(add);
    for (const op of tx.operations) if (op.signer) add(op.signer.key);
  }
  for (const attempt of after.attempts) {
    attempt.signed_by.forEach(add);
    for (const op of attempt.operations) if (op.signer) add(op.signer.key);
  }
  return keys;
}

const SET_OPTIONS_FIELDS = new Set([
  "op",
  "master_weight",
  "low",
  "medium",
  "high",
  "signer",
  "home_domain",
  "set_flags",
  "clear_flags",
]);

function translate(op, keys) {
  if (op.op !== "set_options") throw new Unusable(`operation ${op.op} is not supported`);
  for (const field of Object.keys(op)) {
    if (!SET_OPTIONS_FIELDS.has(field)) throw new Unusable(`set_options field ${field} is not supported`);
  }
  const options = {};
  if (op.master_weight !== undefined) options.masterWeight = op.master_weight;
  if (op.low !== undefined) options.lowThreshold = op.low;
  if (op.medium !== undefined) options.medThreshold = op.medium;
  if (op.high !== undefined) options.highThreshold = op.high;
  if (op.home_domain !== undefined) options.homeDomain = op.home_domain;
  if (op.set_flags !== undefined) options.setFlags = op.set_flags;
  if (op.clear_flags !== undefined) options.clearFlags = op.clear_flags;
  if (op.signer) {
    options.signer = {
      ed25519PublicKey: keys.get(op.signer.key).publicKey(),
      weight: op.signer.weight,
    };
  }
  return Operation.setOptions(options);
}

// The operations that bring a funded default account (master weight 1,
// thresholds 0) to the fixture's state. They are authorized against the
// default state, so the account's own signature is enough.
function setupOperations(snapshot, keys) {
  for (const signer of snapshot.signers) {
    if (signer.type !== "ed25519") throw new Unusable("only ed25519 signers can be replayed");
  }
  const { low, medium, high } = snapshot.thresholds;
  const isDefault =
    snapshot.master_weight === 1 && low === 0 && medium === 0 && high === 0 && snapshot.signers.length === 0;
  if (isDefault) return [];
  return [
    Operation.setOptions({
      masterWeight: snapshot.master_weight,
      lowThreshold: low,
      medThreshold: medium,
      highThreshold: high,
    }),
    ...snapshot.signers.map((signer) =>
      Operation.setOptions({
        signer: { ed25519PublicKey: keys.get(signer.key).publicKey(), weight: signer.weight },
      }),
    ),
  ];
}

// ----------------------------------------------------------------- network

function connect(dryRun) {
  return dryRun ? null : new Horizon.Server(HORIZON);
}

async function fund(address) {
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const response = await fetch(`${FRIENDBOT}/?addr=${address}`);
    if (response.ok) return;
    await sleep(2000);
  }
  throw new Unusable(`Friendbot did not fund ${short(address)}`);
}

async function readRoot() {
  const response = await fetch(HORIZON);
  if (!response.ok) {
    const error = new Unusable(`Horizon answered ${response.status}`);
    error.status = response.status;
    throw error;
  }
  const root = await response.json();
  if (root.network_passphrase !== Networks.TESTNET) {
    throw new Unusable("the server is not Stellar Testnet; refusing to continue");
  }
  return root;
}

// Builds, signs and (unless dry) submits one transaction from `source`.
export async function submit(server, source, operations, signers) {
  let account;
  try {
    account = server ? await server.loadAccount(source.publicKey()) : new Account(source.publicKey(), "0");
  } catch (error) {
    return {
      status: "unconfirmed",
      reason: "account_read_failed_before_transaction_build",
      raw_response: { account_read: errorEvidence(error) },
    };
  }
  let transaction;
  let hash;
  try {
    const builder = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: Networks.TESTNET });
    operations.forEach((operation) => builder.addOperation(operation));
    transaction = builder.setTimeout(120).build();
    signers.forEach((keypair) => transaction.sign(keypair));
    hash = Buffer.from(transaction.hash()).toString("hex");
  } catch (error) {
    return {
      status: "unconfirmed",
      reason: "transaction_build_failed",
      raw_response: { transaction_build: errorEvidence(error) },
    };
  }
  if (!server) return { status: "not_submitted", hash };
  try {
    const response = await server.submitTransaction(transaction);
    if (!response || typeof response !== "object" || Array.isArray(response)) return {
      status: "unconfirmed",
      hash,
      reason: "malformed_submission_response",
      raw_response: { submission: sanitizeHorizonResponse(response) },
    };
    if (response?.hash !== hash) return {
      status: "unconfirmed",
      hash,
      reason: "submission_hash_mismatch",
      returned_hash: response?.hash,
      raw_response: { submission: sanitizeHorizonResponse(response) },
    };
    if (!Number.isSafeInteger(response.ledger) || response.ledger < 1) return {
      status: "unconfirmed",
      hash,
      reason: "malformed_submission_response",
      raw_response: { submission: sanitizeHorizonResponse(response) },
    };
    return { status: "accepted", hash: response.hash, ledger: response.ledger };
  } catch (error) {
    const codes = resultCodesFromSubmission(error);
    if (statusCode(error) !== 400 || !codes) return {
      status: "unconfirmed",
      hash,
      reason: "submission_outcome_unconfirmed",
      raw_response: { submission: errorEvidence(error) },
    };
    return {
      status: "rejected",
      hash,
      codes,
      raw_response: { submission: errorEvidence(error) },
    };
  }
}

class MalformedAccountResponse extends Error {}

function accountState(account) {
  if (!account || typeof account.sequenceNumber !== "function" || !Array.isArray(account.signers) || !account.thresholds) {
    throw new MalformedAccountResponse();
  }
  const sequence = account.sequenceNumber();
  const thresholds = {
    low: account.thresholds.low_threshold,
    medium: account.thresholds.med_threshold,
    high: account.thresholds.high_threshold,
  };
  if (!/^\d+$/.test(String(sequence)) || Object.values(thresholds).some((value) => !Number.isInteger(value) || value < 0 || value > 255)) {
    throw new MalformedAccountResponse();
  }
  const signers = {};
  for (const signer of account.signers) {
    if (!signer || typeof signer.key !== "string" || !Number.isInteger(signer.weight) || signer.weight < 0 || signer.weight > 255 || Object.hasOwn(signers, signer.key)) {
      throw new MalformedAccountResponse();
    }
    signers[signer.key] = signer.weight;
  }
  return {
    sequence: String(sequence),
    thresholds,
    signers: Object.fromEntries(Object.entries(signers).sort(([a], [b]) => a.localeCompare(b))),
  };
}

function sameAuthorizationState(left, right) {
  const normalized = (state) => ({
    thresholds: state.thresholds,
    signers: Object.fromEntries(Object.entries(state.signers).sort(([a], [b]) => a.localeCompare(b))),
  });
  return JSON.stringify(normalized(left)) === JSON.stringify(normalized(right));
}

function expectedAuthorizationState(before, attempt, keys, accountAddress) {
  const state = {
    sequence: before.sequence,
    thresholds: { ...before.thresholds },
    signers: { ...before.signers },
  };
  for (const operation of attempt.operations) {
    if (operation.master_weight !== undefined) state.signers[accountAddress] = operation.master_weight;
    for (const [field, target] of [["low", "low"], ["medium", "medium"], ["high", "high"]]) {
      if (operation[field] !== undefined) state.thresholds[target] = operation[field];
    }
    if (operation.signer) {
      const key = keys.get(operation.signer.key).publicKey();
      if (operation.signer.weight === 0) delete state.signers[key];
      else state.signers[key] = operation.signer.weight;
    }
  }
  return state;
}

function snapshotAuthorizationState(snapshot, keys) {
  const signers = { [keys.get(snapshot.account).publicKey()]: snapshot.master_weight };
  for (const signer of snapshot.signers) signers[keys.get(signer.key).publicKey()] = signer.weight;
  return {
    sequence: null,
    thresholds: {
      low: snapshot.thresholds.low,
      medium: snapshot.thresholds.medium,
      high: snapshot.thresholds.high,
    },
    signers: Object.fromEntries(Object.entries(signers).sort(([a], [b]) => a.localeCompare(b))),
  };
}

function resultCodesFromSubmission(error) {
  const codes = error?.response?.data?.extras?.result_codes;
  if (!codes) return null;
  const normalized = {
    transaction: codes.transaction,
    operations: Array.isArray(codes.operations) ? codes.operations : [],
  };
  return isResultCodes(normalized) ? normalized : null;
}

function statusCode(error) {
  return error?.response?.status ?? error?.status;
}

function errorEvidence(error) {
  const evidence = {};
  const status = statusCode(error);
  if (Number.isInteger(status)) evidence.status_code = status;
  if (typeof error?.name === "string" && /^[A-Za-z][A-Za-z0-9]{0,63}$/.test(error.name)) evidence.error_name = error.name;
  if (typeof error?.code === "string" && /^[A-Z0-9_-]{1,64}$/i.test(error.code)) evidence.error_code = error.code;
  if (error?.response?.data !== undefined) evidence.response = sanitizeHorizonResponse(error.response.data);
  return evidence;
}

async function lookupTransaction(server, hash, { sleep: wait = sleep, checks = HISTORY_CHECKS, delayMs = HISTORY_CHECK_DELAY_MS } = {}) {
  let last404 = false;
  for (let check = 0; check < checks; check += 1) {
    try {
      const record = await server.transactions().transaction(hash).call();
      if (!record || typeof record !== "object" || Array.isArray(record)) {
        return {
          record: null,
          error: { response: sanitizeHorizonResponse(record) },
          errorKind: "malformed_history_response",
          checks: check + 1,
        };
      }
      return { record, error: null, checks: check + 1 };
    } catch (error) {
      if (statusCode(error) !== 404) return {
        record: null,
        error: errorEvidence(error),
        errorKind: "history_lookup_failed",
        checks: check + 1,
      };
      last404 = true;
      if (check + 1 < checks) await wait(delayMs);
    }
  }
  return { record: null, error: null, checks, last404 };
}

async function readAccountState(server, address) {
  try {
    return { state: accountState(await server.loadAccount(address)), error: null };
  } catch (error) {
    return {
      state: null,
      error: errorEvidence(error),
      reason: error instanceof MalformedAccountResponse ? "malformed_account_response" : "account_read_failed",
    };
  }
}

async function waitForAccountState(server, address, predicate, { sleep: wait = sleep, checks = HISTORY_CHECKS, delayMs = HISTORY_CHECK_DELAY_MS } = {}) {
  let state = null;
  let readError = null;
  let readFailure = null;
  const readFailures = [];
  for (let check = 0; check < checks; check += 1) {
    const read = await readAccountState(server, address);
    if (read.error) {
      readError = read.error;
      readFailure = read.reason;
      readFailures.push({ check: check + 1, reason: read.reason, evidence: read.error });
    } else {
      state = read.state;
      readError = null;
      readFailure = null;
      try {
        if (predicate(state)) return {
          state, matched: true, checks: check + 1, error: null, reason: null, readFailures,
        };
      } catch {
        return { state, matched: false, checks: check + 1, error: null,
          reason: "account_state_comparison_failed", readFailures };
      }
    }
    if (check + 1 < checks) await wait(delayMs);
  }
  return { state, matched: false, checks, error: readError, reason: readFailure, readFailures };
}

export async function replayAfterAttempt(server, account, attempt, keys, dependencies = {}) {
  const progress = dependencies.progress ?? { raw_response: {} };
  progress.raw_response ??= {};
  const waitOptions = {
    sleep: dependencies.sleep ?? sleep,
    checks: dependencies.historyChecks ?? HISTORY_CHECKS,
    delayMs: dependencies.historyCheckDelayMs ?? HISTORY_CHECK_DELAY_MS,
  };
  const accountAddress = account.publicKey();
  if (!server) {
    const observed = await submit(null, account, attempt.operations.map((op) => translate(op, keys)),
      attempt.signed_by.map((address) => keys.get(address)));
    return {
      id: attempt.id,
      expected: attempt.expected,
      ...observed,
      match: null,
    };
  }

  const base = { id: attempt.id, expected: attempt.expected, status: "unconfirmed", match: null };
  const beforeRead = await readAccountState(server, accountAddress);
  if (beforeRead.error) {
    progress.raw_response.account_read = beforeRead.error;
    return {
      ...base,
      reason: beforeRead.reason,
      attempted: false,
      raw_response: progress.raw_response,
    };
  }
  const beforeSubmitState = beforeRead.state;
  progress.authorization_state_before = beforeSubmitState;
  const signers = attempt.signed_by.map((address) => keys.get(address));
  if (signers.some((key) => !key)) return { ...base, reason: "signing_key_unavailable", attempted: false };

  let transaction;
  let hash;
  try {
    const operations = attempt.operations.map((op) => translate(op, keys));
    const accountForTx = new Account(accountAddress, beforeSubmitState.sequence);
    const builder = new TransactionBuilder(accountForTx, { fee: BASE_FEE, networkPassphrase: Networks.TESTNET });
    operations.forEach((operation) => builder.addOperation(operation));
    transaction = builder.setTimeout(120).build();
    signers.forEach((keypair) => transaction.sign(keypair));
    hash = Buffer.from(transaction.hash()).toString("hex");
    progress.hash = hash;
  } catch (error) {
    progress.raw_response.transaction_build = errorEvidence(error);
    return { ...base, reason: "transaction_build_failed", attempted: false,
      raw_response: progress.raw_response };
  }

  let submission;
  try {
    const response = await server.submitTransaction(transaction);
    submission = {
      kind: "accepted",
      hash: response?.hash,
      returned_ledger: response?.ledger,
      raw: sanitizeHorizonResponse(response),
    };
    submission.hash_mismatch = response?.hash !== hash;
  } catch (error) {
    const resultCodes = resultCodesFromSubmission(error);
    submission = {
      kind: statusCode(error) === 400 && resultCodes ? "rejected" : "uncertain",
      result_codes: statusCode(error) === 400 ? resultCodes : null,
      raw: errorEvidence(error),
    };
  }
  progress.raw_response.submission = submission.raw;

  const history = await lookupTransaction(server, hash, waitOptions);
  const raw = { submission: submission.raw };
  progress.raw_response = raw;
  let status;
  let resultCodes;
  let ledger;
  let historyRaw;
  let accountAfterRejection;
  if (history.record) {
    historyRaw = sanitizeHorizonResponse(history.record);
    raw.history = historyRaw;
    if (submission.hash_mismatch) return {
      ...base, reason: "submission_hash_mismatch", hash, returned_hash: submission.hash,
      history_checks: history.checks, authorization_state_before: beforeSubmitState,
      raw_response: raw,
    };
    if (history.record.hash !== hash || !Number.isSafeInteger(history.record.ledger_attr) || history.record.ledger_attr < 1) {
      return { ...base, reason: "invalid_history_identity_or_ledger", hash,
        history_checks: history.checks, authorization_state_before: beforeSubmitState, raw_response: raw };
    }
    if (submission.returned_ledger !== undefined && submission.returned_ledger !== history.record.ledger_attr) {
      return { ...base, reason: "submission_ledger_mismatch", hash,
        returned_ledger: submission.returned_ledger, history_ledger: history.record.ledger_attr,
        history_checks: history.checks, authorization_state_before: beforeSubmitState, raw_response: raw };
    }
    const classified = classifyAttemptOutcome({ record: history.record });
    if (classified.status === "unconfirmed") return { ...base, reason: classified.reason, hash,
      history_checks: history.checks, authorization_state_before: beforeSubmitState, raw_response: raw };
    if (submission.kind === "rejected" && !compareResultCodes(classified.result_codes, submission.result_codes)) {
      return { ...base, reason: "submission_history_result_mismatch", hash,
        history_checks: history.checks, authorization_state_before: beforeSubmitState, raw_response: raw };
    }
    status = classified.status;
    resultCodes = classified.result_codes;
    ledger = history.record.ledger_attr;
  } else if (history.error) {
    raw.history_lookup = history.error;
    progress.raw_response = raw;
    return {
      ...base,
      reason: history.errorKind ?? "history_lookup_failed",
      hash,
      ...(submission.result_codes ? { result_codes: submission.result_codes } : {}),
      history_checks: history.checks,
      authorization_state_before: beforeSubmitState,
      raw_response: raw,
    };
  } else if (history.last404 && submission.kind === "rejected" && submission.result_codes) {
    const reconciliation = await waitForAccountState(server, accountAddress, (state) =>
      state.sequence === beforeSubmitState.sequence && sameAuthorizationState(beforeSubmitState, state), waitOptions);
    if (!reconciliation.state) {
      if (reconciliation.error) raw.account_reconciliation = reconciliation.error;
      return {
        ...base,
        reason: reconciliation.reason ?? "account_reconciliation_failed",
        hash,
        result_codes: submission.result_codes,
        authorization_state_before: beforeSubmitState,
        non_inclusion_checks: history.checks,
        non_inclusion_is_absolute_proof: false,
        raw_response: raw,
      };
    }
    accountAfterRejection = reconciliation.state;
    if (reconciliation.readFailures.length) raw.account_read_failures = reconciliation.readFailures;
    if (!reconciliation.matched) {
      return {
        ...base,
        reason: "account_state_or_sequence_mismatch",
        hash,
        authorization_state_before: beforeSubmitState,
        authorization_state_after: reconciliation.state,
        sequence_before: beforeSubmitState.sequence,
        sequence_after: reconciliation.state.sequence,
        result_codes: submission.result_codes,
        non_inclusion_checks: history.checks,
        non_inclusion_is_absolute_proof: false,
        raw_response: raw,
      };
    }
    status = "rejected_at_submission";
    resultCodes = submission.result_codes;
  } else {
    return {
      ...base,
      reason: history.last404 ? "history_not_found_after_bounded_checks" : "submission_outcome_unconfirmed",
      hash,
      ...(submission.result_codes ? { result_codes: submission.result_codes } : {}),
      authorization_state_before: beforeSubmitState,
      non_inclusion_checks: history.checks,
      non_inclusion_is_absolute_proof: false,
      raw_response: raw,
    };
  }

  const seqBefore = BigInt(beforeSubmitState.sequence);
  const wantedState = status === "accepted"
    ? expectedAuthorizationState(beforeSubmitState, attempt, keys, accountAddress)
    : beforeSubmitState;
  const expectedSequence = status === "rejected_at_submission" ? seqBefore : seqBefore + 1n;
  const after = accountAfterRejection
    ? { state: accountAfterRejection, matched: true, checks: 1, error: null, reason: null }
    : await waitForAccountState(server, accountAddress, (state) =>
      BigInt(state.sequence) === expectedSequence && sameAuthorizationState(wantedState, state), waitOptions);
  if (after.error) raw.account_reconciliation = after.error;
  if (after.readFailures?.length) raw.account_read_failures = after.readFailures;
  if (after.state) progress.authorization_state_after = after.state;
  if (resultCodes) progress.result_codes = resultCodes;
  if (ledger !== undefined) progress.ledger = ledger;
  progress.observed_ledger_outcome = status;
  progress.raw_response = raw;
  if (!after.state) return {
    ...base,
    reason: after.reason ?? "account_reconciliation_failed",
    hash,
    ledger,
    result_codes: resultCodes,
    history_checks: history.checks,
    sequence_before: beforeSubmitState.sequence,
    sequence_after: null,
    authorization_state_before: beforeSubmitState,
    raw_response: raw,
    ...(ledger === undefined ? {} : { observed_ledger_outcome: status }),
  };
  if (!after.matched) return {
    ...base,
    reason: after.error ? after.reason ?? "account_reconciliation_failed" : "account_state_or_sequence_mismatch",
    hash,
    ledger,
    result_codes: resultCodes,
    history_checks: history.checks,
    sequence_before: beforeSubmitState.sequence,
    sequence_after: after.state.sequence,
    authorization_state_before: beforeSubmitState,
    authorization_state_after: after.state,
    raw_response: raw,
    ...(ledger === undefined ? {} : { observed_ledger_outcome: status }),
  };

  const matched = status === attempt.expected.stage && compareResultCodes(resultCodes, attempt.expected.result_codes);
  return {
    ...base,
    status,
    hash,
    ...(ledger === undefined ? {} : { ledger }),
    history_checks: history.checks,
    sequence_before: beforeSubmitState.sequence,
    sequence_after: after.state.sequence,
    authorization_state_before: beforeSubmitState,
    authorization_state_after: after.state,
    result_codes: resultCodes,
    ...(status === "rejected_at_submission" ? {
      non_inclusion_checks: history.checks,
      non_inclusion_is_absolute_proof: false,
    } : {}),
    raw_response: raw,
    match: matched,
  };
}

export async function runFollowupAttempts(server, account, attempts, keys, dependencies = {}) {
  const results = [];
  for (const attempt of attempts) {
    const progress = { raw_response: {} };
    let observed;
    try {
      observed = await replayAfterAttempt(server, account, attempt, keys, { ...dependencies, progress });
    } catch (error) {
      observed = {
        id: attempt.id,
        expected: attempt.expected,
        status: "unconfirmed",
        reason: "followup_execution_failed",
        ...(progress.hash ? { hash: progress.hash } : {}),
        ...(progress.ledger !== undefined ? { ledger: progress.ledger } : {}),
        ...(progress.result_codes ? { result_codes: progress.result_codes } : {}),
        ...(progress.authorization_state_before ? { authorization_state_before: progress.authorization_state_before } : {}),
        ...(progress.authorization_state_after ? { authorization_state_after: progress.authorization_state_after } : {}),
        raw_response: {
          ...progress.raw_response,
          execution_error: errorEvidence(error),
        },
        match: null,
      };
    }
    results.push(observed);
    if (server && observed.match !== true) {
      results.push(...attempts.slice(results.length).map((later) => ({
        id: later.id,
        expected: later.expected,
        status: "not_run",
        reason: observed.status === "unconfirmed" ? "prior_attempt_unconfirmed" : "prior_attempt_did_not_match",
        match: null,
      })));
      break;
    }
  }
  return results;
}

export function replayExitCode(results, differing) {
  const unconfirmed = results.some((result) => result.status === "unconfirmed" || result.incomplete ||
    result.after?.some((attempt) => ["unconfirmed", "not_run"].includes(attempt.status)) ||
    result.transactions?.some((attempt) => ["unconfirmed", "not_run"].includes(attempt.status)) ||
    ["unconfirmed", "not_run"].includes(result.setup?.status) ||
    ["unconfirmed", "not_run"].includes(result.probe?.status));
  return unconfirmed ? 2 : differing.length === 0 ? 0 : 1;
}

async function checkBuiltState(server, account, snapshot, keys) {
  const loaded = await server.loadAccount(account.publicKey());
  const wanted = snapshot.signers
    .map((signer) => `${keys.get(signer.key).publicKey()}:${signer.weight}`)
    .sort();
  const found = loaded.signers
    .filter((signer) => signer.key !== account.publicKey())
    .map((signer) => `${signer.key}:${signer.weight}`)
    .sort();
  const master = loaded.signers.find((signer) => signer.key === account.publicKey());
  const same =
    loaded.thresholds.low_threshold === snapshot.thresholds.low &&
    loaded.thresholds.med_threshold === snapshot.thresholds.medium &&
    loaded.thresholds.high_threshold === snapshot.thresholds.high &&
    (master ? master.weight : 0) === snapshot.master_weight &&
    JSON.stringify(wanted) === JSON.stringify(found);
  if (!same) throw new Unusable("the account built on Testnet does not match the fixture's snapshot");
}

// ------------------------------------------------------------------ replay

export async function replay(fixture, server, { fundAccount = fund } = {}) {
  const { name, expected, snapshot, plan } = fixture;
  if (snapshot.network !== "testnet") throw new Unusable(`${name}: the snapshot is not for testnet`);
  const keys = keypairsFor(fixture);
  const account = keys.get(snapshot.account);
  const result = { fixture: expected.id, directory: name, account: account.publicKey() };
  result.transactions = [];
  result.after = [];
  let activeStage = "funding";
  let activeTransaction = null;

  try {
    if (server) await fundAccount(account.publicKey());

    activeStage = "setup";
    const setup = setupOperations(snapshot, keys);
    result.setup = setup.length === 0 ? null : await submit(server, account, setup, [account]);
    if (result.setup && server && result.setup.status !== "accepted") {
      const status = result.setup.status;
      result.transactions = plan.transactions.map((tx, index) => ({
        id: tx.id,
        expected: expected.transactions[index],
        status: "not_run",
        reason: "setup_transaction_did_not_confirm_acceptance",
        match: null,
      }));
      result.after = fixture.after.attempts.map((attempt) => ({
        id: attempt.id,
        expected: attempt.expected,
        status: "not_run",
        reason: "setup_transaction_did_not_confirm_acceptance",
        match: null,
      }));
      result.match = status === "unconfirmed" ? null : false;
      return result;
    }
    if (server) {
      activeStage = "setup_reconciliation";
      await checkBuiltState(server, account, snapshot, keys);
    }
    const setupStateRead = server ? await readAccountState(server, account.publicKey()) : null;
    const setupState = setupStateRead?.state ?? null;

    for (const [index, tx] of plan.transactions.entries()) {
      activeStage = "planned_transaction";
      activeTransaction = { index, id: tx.id };
      if (tx.source !== undefined) throw new Unusable(`${name}: a transaction source is not supported`);
      const operations = tx.operations.map((op) => translate(op, keys));
      const signers = tx.signed_by.map((address) => keys.get(address));
      const observed = await submit(server, account, operations, signers);
      const wanted = expected.transactions[index];
      result.transactions.push({
        id: tx.id,
        expected: wanted,
        ...observed,
        match: server ? (observed.status === "unconfirmed" ? null : observed.status === wanted) : null,
      });
      if (server && observed.status === "unconfirmed") {
        result.transactions.push(...plan.transactions.slice(index + 1).map((later, offset) => ({
          id: later.id,
          expected: expected.transactions[index + 1 + offset],
          status: "not_run",
          reason: "prior_transaction_unconfirmed",
          match: null,
        })));
        break;
      }
    }

    result.after = [];
    if (fixture.after.attempts.length > 0) {
      activeStage = "after_precondition";
      activeTransaction = null;
      let planMatches = server && expected.transactions.every((transaction) => transaction === "accepted") &&
        result.transactions.every((transaction) => transaction.match === true);
      if (planMatches) {
        if (!setupState) {
          planMatches = false;
          result.after_precondition = {
            status: "unconfirmed",
            reason: setupStateRead?.reason ?? "account_state_unavailable",
            raw_response: { account_read: setupStateRead?.error },
          };
        } else {
          let plannedState = snapshotAuthorizationState(snapshot, keys);
          plannedState.sequence = setupState.sequence;
          for (const tx of plan.transactions) plannedState = expectedAuthorizationState(plannedState, tx, keys, account.publicKey());
          plannedState.sequence = (BigInt(setupState.sequence) + BigInt(plan.transactions.length)).toString();
          const reconciliation = await waitForAccountState(server, account.publicKey(), (state) =>
            state.sequence === plannedState.sequence && sameAuthorizationState(plannedState, state));
          planMatches = reconciliation.matched;
          result.after_precondition = {
            status: planMatches ? "matched" : "unconfirmed",
            reason: reconciliation.reason ?? (planMatches ? undefined : "planned_account_state_not_reconciled"),
            expected_sequence: plannedState.sequence,
            observed_sequence: reconciliation.state?.sequence ?? null,
            observed_account_state: reconciliation.state,
            ...(reconciliation.error || reconciliation.readFailures?.length ? {
              raw_response: {
                ...(reconciliation.error ? { account_reconciliation: reconciliation.error } : {}),
                ...(reconciliation.readFailures?.length ? { account_read_failures: reconciliation.readFailures } : {}),
              },
            } : {}),
          };
        }
      }
      if (server && !planMatches) {
        result.after = fixture.after.attempts.map((attempt) => ({
          id: attempt.id,
          expected: attempt.expected,
          status: "not_run",
          reason: "planned_transactions_or_post_plan_state_did_not_match",
          match: null,
        }));
      } else {
        activeStage = "followup_attempts";
        result.after = await runFollowupAttempts(server, account, fixture.after.attempts, keys);
      }
    }

    // A fixture with no transactions is a state check. The probe submits a
    // change-nothing transaction signed by every signer. The stated rule is that
    // an envelope needs the source account's LOW threshold, so it is accepted
    // only if the summed weights reach it.
    if (plan.transactions.length === 0) {
      activeStage = "probe";
      const total = Math.min(
        snapshot.signers.reduce((sum, signer) => sum + Math.min(signer.weight, 255), snapshot.master_weight),
        255,
      );
      const wanted = total >= snapshot.thresholds.low ? "accepted" : "rejected";
      const everyone = [account, ...snapshot.signers.map((signer) => keys.get(signer.key))];
      const observed = await submit(server, account, [Operation.setOptions({})], everyone);
      result.probe = {
        rule: "the envelope needs the source account's LOW threshold",
        expected: wanted,
        ...observed,
        match: server ? (observed.status === "unconfirmed" ? null : observed.status === wanted) : null,
      };
    }

    const checks = [...result.transactions, ...(result.probe ? [result.probe] : []), ...result.after];
    result.match = server
      ? checks.some((check) => check.match === false)
        ? false
        : checks.every((check) => check.match === true)
          ? true
          : null
      : null;
    return result;
  } catch (error) {
    result.status = "unconfirmed";
    result.incomplete = { stage: activeStage, reason: "fixture_replay_failed", evidence: errorEvidence(error) };
    if (activeStage === "planned_transaction" && activeTransaction) {
      if (!result.transactions.some((transaction) => transaction.id === activeTransaction.id)) {
        result.transactions.push({
          id: activeTransaction.id,
          expected: expected.transactions[activeTransaction.index],
          status: "unconfirmed",
          reason: "transaction_outcome_unavailable",
          evidence: errorEvidence(error),
          match: null,
        });
      }
      result.transactions.push(...plan.transactions.slice(activeTransaction.index + 1).map((transaction, offset) => ({
        id: transaction.id,
        expected: expected.transactions[activeTransaction.index + 1 + offset],
        status: "not_run",
        reason: "prior_transaction_unconfirmed",
        match: null,
      })));
    }
    const recordedTransactions = new Set(result.transactions.map((transaction) => transaction.id));
    result.transactions.push(...plan.transactions.map((transaction, index) => ({ transaction, index }))
      .filter(({ transaction }) => !recordedTransactions.has(transaction.id))
      .map(({ transaction, index }) => ({
        id: transaction.id,
        expected: expected.transactions[index],
        status: "not_run",
        reason: "fixture_replay_failed",
        match: null,
      })));
    if (activeStage === "setup" && !result.setup) {
      result.setup = { status: "unconfirmed", reason: "setup_outcome_unavailable", evidence: errorEvidence(error) };
    }
    const recordedFollowups = new Set(result.after.map((attempt) => attempt.id));
    result.after.push(...fixture.after.attempts.filter((attempt) => !recordedFollowups.has(attempt.id)).map((attempt) => ({
      id: attempt.id,
      expected: attempt.expected,
      status: "not_run",
      reason: "fixture_replay_failed",
      match: null,
    })));
    if (activeStage === "probe") {
      result.probe = { status: "unconfirmed", reason: "probe_outcome_unavailable", evidence: errorEvidence(error), match: null };
    }
    result.match = null;
    return result;
  }
}

function describe(result) {
  const checks = [...result.transactions, ...(result.probe ? [{ id: "probe", ...result.probe }] : [])];
  for (const check of checks) {
    const where = check.ledger ? ` ledger ${check.ledger}` : "";
    const mark = check.match === null ? "-" : check.match ? "ok" : "DIFFERS";
    log(`  ${check.id}: expected ${check.expected}, observed ${check.status}${where} [${mark}]`);
  }
  for (const attempt of result.after ?? []) {
    const where = attempt.ledger ? ` ledger ${attempt.ledger}` : "";
    const mark = attempt.match === null ? "-" : attempt.match ? "ok" : "DIFFERS";
    log(`  after ${attempt.id}: expected ${attempt.expected?.stage ?? "not run"}, observed ${attempt.status}${where} [${mark}]`);
  }
}

function incompleteFixture(fixture, stage, error) {
  return {
    fixture: fixture.expected.id,
    directory: fixture.name,
    status: "unconfirmed",
    incomplete: { stage, reason: "fixture_replay_failed", evidence: errorEvidence(error) },
    transactions: fixture.plan.transactions.map((transaction, index) => ({
      id: transaction.id,
      expected: fixture.expected.transactions[index],
      status: "not_run",
      reason: "fixture_replay_failed",
      match: null,
    })),
    after: fixture.after.attempts.map((attempt) => ({
      id: attempt.id,
      expected: attempt.expected,
      status: "not_run",
      reason: "fixture_replay_failed",
      match: null,
    })),
    match: null,
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const fixtures = loadChainFixtures(options.only);
  const server = connect(options.dryRun);

  const meta = { tool: "signerreach-replay/1", sdk: SDK_VERSION, horizon: HORIZON, dry_run: options.dryRun };
  meta.started = new Date().toISOString();
  let preflightFailure = null;
  if (server) {
    try {
      const root = await readRoot();
      meta.protocol = root.current_protocol_version;
      meta.core = root.core_version;
      meta.ledger_at_start = root.history_latest_ledger;
      const fixtureProtocols = [...new Set(fixtures.map((f) => f.snapshot.provenance.protocol_version))];
      meta.fixture_protocols = fixtureProtocols;
      if (meta.protocol !== 29 || fixtureProtocols.some((p) => p !== meta.protocol)) {
        throw new Unusable("live replay requires protocol 29 fixtures on a protocol 29 Testnet");
      }
      log(`Testnet protocol ${meta.protocol}, ${meta.core}, ledger ${meta.ledger_at_start}`);
    } catch (error) {
      preflightFailure = error;
    }
  }

  const results = [];
  for (const fixture of fixtures) {
    log(`${fixture.expected.id} (${fixture.name})`);
    let result;
    if (preflightFailure) result = incompleteFixture(fixture, "network_preflight", preflightFailure);
    else {
      try {
        result = await replay(fixture, server);
      } catch (error) {
        result = incompleteFixture(fixture, "fixture_replay", error);
      }
    }
    describe(result);
    results.push(result);
  }
  meta.finished = new Date().toISOString();

  const differing = results.filter((result) => result.match === false).map((result) => result.fixture);
  const exitCode = replayExitCode(results, differing);
  const document = JSON.stringify({ meta, results, differing }, null, 2) + "\n";
  if (options.out) writeFileSync(options.out, document);
  else process.stdout.write(document);

  if (options.dryRun) log(`dry run: ${results.length} fixtures built and signed; nothing was sent`);
  else if (exitCode === 2) log("one or more outcomes were unconfirmed or not run");
  else if (differing.length === 0) log(`all ${results.length} fixtures matched`);
  else log(`DIFFERED: ${differing.join(", ")}`);
  return exitCode;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) main().then(
  (code) => {
    process.exitCode = code;
  },
  (error) => {
    log(`replay: ${error instanceof Unusable ? error.message : "unexpected failure"}`);
    process.exitCode = 2;
  },
);
