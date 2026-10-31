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
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
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
      afterText: existsSync(afterPath)
        ? readFileSync(afterPath, "utf8")
        : JSON.stringify({ schema: "signerreach.replay-after/1", attempts: [] }),
    });
  }
  if (fixtures.length === 0) throw new Unusable("no chain fixtures matched");
  for (const fixture of fixtures) validateReplayAfter(fixture);
  return fixtures;
}

function validateReplayAfter(fixture) {
  const { snapshot, plan, availability, afterText, name } = fixture;
  const knownAddresses = new Set([snapshot.account]);
  for (const signer of snapshot.signers) knownAddresses.add(signer.key);
  for (const tx of plan.transactions) {
    tx.signed_by.forEach((address) => knownAddresses.add(address));
    for (const op of tx.operations) if (op.signer?.key) knownAddresses.add(op.signer.key);
  }
  const statusOf = (address) => availability.signers?.[address] ?? availability.default ?? "unknown";
  const { document, errors } = parseAfterJson(afterText, {
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
  if (!response.ok) throw new Unusable(`Horizon answered ${response.status}`);
  const root = await response.json();
  if (root.network_passphrase !== Networks.TESTNET) {
    throw new Unusable("the server is not Stellar Testnet; refusing to continue");
  }
  return root;
}

// Builds, signs and (unless dry) submits one transaction from `source`.
async function submit(server, source, operations, signers) {
  const account = server ? await server.loadAccount(source.publicKey()) : new Account(source.publicKey(), "0");
  const builder = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: Networks.TESTNET });
  operations.forEach((operation) => builder.addOperation(operation));
  const transaction = builder.setTimeout(120).build();
  signers.forEach((keypair) => transaction.sign(keypair));
  const hash = Buffer.from(transaction.hash()).toString("hex");
  if (!server) return { status: "not_submitted", hash };
  try {
    const response = await server.submitTransaction(transaction);
    return { status: "accepted", hash: response.hash, ledger: response.ledger };
  } catch (error) {
    const codes = error?.response?.data?.extras?.result_codes;
    if (!codes) throw new Unusable("submission failed without a result");
    return { status: "rejected", hash, codes };
  }
}

function accountState(account) {
  const signers = Object.fromEntries(account.signers.map(({ key, weight }) => [key, weight]));
  return {
    sequence: account.sequenceNumber(),
    thresholds: {
      low: account.thresholds.low_threshold,
      medium: account.thresholds.med_threshold,
      high: account.thresholds.high_threshold,
    },
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

async function lookupTransaction(server, hash) {
  let last404 = false;
  for (let check = 0; check < HISTORY_CHECKS; check += 1) {
    try {
      const record = await server.transactions().transaction(hash).call();
      return { record, error: null, checks: check + 1 };
    } catch (error) {
      if (statusCode(error) !== 404) return { record: null, error, checks: check + 1 };
      last404 = true;
      if (check + 1 < HISTORY_CHECKS) await sleep(HISTORY_CHECK_DELAY_MS);
    }
  }
  return { record: null, error: null, checks: HISTORY_CHECKS, last404 };
}

async function readAccountState(server, address) {
  return accountState(await server.loadAccount(address));
}

async function waitForAccountState(server, address, predicate) {
  let state;
  for (let check = 0; check < HISTORY_CHECKS; check += 1) {
    state = await readAccountState(server, address);
    if (predicate(state)) return state;
    if (check + 1 < HISTORY_CHECKS) await sleep(HISTORY_CHECK_DELAY_MS);
  }
  return state;
}

async function replayAfterAttempt(server, account, attempt, keys) {
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

  const loadedAccount = await server.loadAccount(accountAddress);
  const signers = attempt.signed_by.map((address) => keys.get(address));
  if (signers.some((key) => !key)) throw new Unusable(`${attempt.id}: a signing key is unavailable`);

  let submission;
  const operations = attempt.operations.map((op) => translate(op, keys));
  // Use this exact loaded record so the state comparison and transaction use
  // one sequence.
  const beforeSubmitState = accountState(loadedAccount);
  const accountForTx = new Account(accountAddress, loadedAccount.sequenceNumber());
  const builder = new TransactionBuilder(accountForTx, { fee: BASE_FEE, networkPassphrase: Networks.TESTNET });
  operations.forEach((operation) => builder.addOperation(operation));
  const transaction = builder.setTimeout(120).build();
  signers.forEach((keypair) => transaction.sign(keypair));
  const hash = Buffer.from(transaction.hash()).toString("hex");
  try {
    const response = await server.submitTransaction(transaction);
    if (response.hash !== hash) {
      return { id: attempt.id, status: "unconfirmed", reason: "submission_hash_mismatch", hash,
        returned_hash: response.hash, raw_response: { submission: sanitizeHorizonResponse(response) }, match: null };
    }
    submission = {
      accepted: true,
      hash: response.hash,
      returned_ledger: response.ledger,
      raw: sanitizeHorizonResponse(response),
    };
  } catch (error) {
    const resultCodes = resultCodesFromSubmission(error);
    if (!resultCodes) {
      return { id: attempt.id, status: "unconfirmed", reason: "submission_error_without_result_codes", hash,
        raw_response: { submission: sanitizeHorizonResponse(error?.response?.data) }, match: null };
    }
    submission = {
      accepted: false,
      hash,
      result_codes: resultCodes,
      raw: sanitizeHorizonResponse(error.response.data),
    };
  }

  const history = await lookupTransaction(server, hash);
  let status;
  let resultCodes;
  let ledger;
  let historyRaw;
  if (history.record) {
    if (history.record.hash !== hash || !Number.isSafeInteger(history.record.ledger_attr) || history.record.ledger_attr < 1) {
      return { id: attempt.id, status: "unconfirmed", reason: "invalid_history_identity_or_ledger", hash,
        raw_response: { submission: submission.raw, history: sanitizeHorizonResponse(history.record) }, match: null };
    }
    if (submission.returned_ledger !== undefined && submission.returned_ledger !== history.record.ledger_attr) {
      return { id: attempt.id, status: "unconfirmed", reason: "submission_ledger_mismatch", hash,
        returned_ledger: submission.returned_ledger, history_ledger: history.record.ledger_attr,
        raw_response: { submission: submission.raw, history: sanitizeHorizonResponse(history.record) }, match: null };
    }
    const classified = classifyAttemptOutcome({ record: history.record });
    if (classified.status === "unconfirmed") {
      return { id: attempt.id, status: "unconfirmed", reason: classified.reason, hash,
        raw_response: { submission: submission.raw, history: sanitizeHorizonResponse(history.record) }, match: null };
    }
    status = classified.status;
    resultCodes = classified.result_codes;
    ledger = history.record.ledger_attr;
    historyRaw = sanitizeHorizonResponse(history.record);
    if (status === "rejected_at_submission") status = "unconfirmed";
  } else {
    let accountUnchanged = false;
    let stateAfterRejection;
    if (!submission.accepted && history.last404) {
      stateAfterRejection = await waitForAccountState(server, accountAddress, (state) =>
        state.sequence === beforeSubmitState.sequence && sameAuthorizationState(beforeSubmitState, state));
      accountUnchanged = stateAfterRejection.sequence === beforeSubmitState.sequence &&
        sameAuthorizationState(beforeSubmitState, stateAfterRejection);
    }
    const classified = classifyAttemptOutcome({
      historyError: Boolean(history.error),
      historyAbsent: Boolean(history.last404),
      submissionResultCodes: submission.result_codes,
      accountUnchanged,
    });
    if (classified.status === "unconfirmed") {
      return {
        id: attempt.id,
        status: "unconfirmed",
        reason: classified.reason,
        hash,
        ...(stateAfterRejection ? {
          sequence_before: beforeSubmitState.sequence,
          sequence_after: stateAfterRejection.sequence,
        } : {}),
        non_inclusion_checks: history.checks,
        non_inclusion_is_absolute_proof: false,
        result_codes: submission.result_codes,
        raw_response: { submission: submission.raw },
        match: null,
      };
    }
    status = classified.status;
    resultCodes = classified.result_codes;
  }

  const seqBefore = BigInt(beforeSubmitState.sequence);
  const wantedState = status === "accepted"
    ? expectedAuthorizationState(beforeSubmitState, attempt, keys, accountAddress)
    : beforeSubmitState;
  const expectedSequence = status === "rejected_at_submission" ? seqBefore : seqBefore + 1n;
  const after = await waitForAccountState(server, accountAddress, (state) =>
    BigInt(state.sequence) === expectedSequence && sameAuthorizationState(wantedState, state));
  const seqAfter = BigInt(after.sequence);
  const sequenceOkay = seqAfter === expectedSequence;
  const stateOkay = sameAuthorizationState(wantedState, after);
  if (!sequenceOkay || !stateOkay) {
    return { id: attempt.id, status: "unconfirmed", reason: "account_state_or_sequence_mismatch", hash,
      ledger, sequence_before: beforeSubmitState.sequence, sequence_after: after.sequence,
      result_codes: resultCodes,
      raw_response: { submission: submission.raw, history: historyRaw }, match: null };
  }

  const expectedStage = attempt.expected.stage;
  const codesOkay = compareResultCodes(resultCodes, attempt.expected.result_codes);
  const matched = status === expectedStage && codesOkay;
  return {
    id: attempt.id,
    expected: attempt.expected,
    status,
    hash,
    ...(ledger === undefined ? {} : { ledger }),
    sequence_before: beforeSubmitState.sequence,
    sequence_after: after.sequence,
    result_codes: resultCodes,
    ...(status === "rejected_at_submission" ? {
      non_inclusion_checks: history.checks,
      non_inclusion_is_absolute_proof: false,
    } : {}),
    raw_response: { submission: submission.raw, ...(historyRaw ? { history: historyRaw } : {}) },
    match: matched,
  };
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

async function replay(fixture, server) {
  const { name, expected, snapshot, plan } = fixture;
  if (snapshot.network !== "testnet") throw new Unusable(`${name}: the snapshot is not for testnet`);
  const keys = keypairsFor(fixture);
  const account = keys.get(snapshot.account);
  const result = { fixture: expected.id, directory: name, account: account.publicKey() };

  if (server) await fund(account.publicKey());

  const setup = setupOperations(snapshot, keys);
  result.setup = setup.length === 0 ? null : await submit(server, account, setup, [account]);
  if (result.setup && server && result.setup.status !== "accepted") {
    throw new Unusable(`${name}: the setup transaction was not accepted`);
  }
  if (server) await checkBuiltState(server, account, snapshot, keys);
  const setupState = server ? await readAccountState(server, account.publicKey()) : null;

  result.transactions = [];
  for (const [index, tx] of plan.transactions.entries()) {
    if (tx.source !== undefined) throw new Unusable(`${name}: a transaction source is not supported`);
    const operations = tx.operations.map((op) => translate(op, keys));
    const signers = tx.signed_by.map((address) => keys.get(address));
    const observed = await submit(server, account, operations, signers);
    const wanted = expected.transactions[index];
    result.transactions.push({
      id: tx.id,
      expected: wanted,
      ...observed,
      match: server ? observed.status === wanted : null,
    });
  }

  result.after = [];
  if (fixture.after.attempts.length > 0) {
    let planMatches = server && expected.transactions.every((transaction) => transaction === "accepted") &&
      result.transactions.every((transaction) => transaction.match === true);
    if (planMatches) {
      let plannedState = snapshotAuthorizationState(snapshot, keys);
      plannedState.sequence = setupState.sequence;
      for (const tx of plan.transactions) plannedState = expectedAuthorizationState(plannedState, tx, keys, account.publicKey());
      plannedState.sequence = (BigInt(setupState.sequence) + BigInt(plan.transactions.length)).toString();
      const actualState = await waitForAccountState(server, account.publicKey(), (state) =>
        state.sequence === plannedState.sequence && sameAuthorizationState(plannedState, state));
      planMatches = actualState.sequence === plannedState.sequence && sameAuthorizationState(plannedState, actualState);
      result.after_precondition = {
        status: planMatches ? "matched" : "unconfirmed",
        expected_sequence: plannedState.sequence,
        observed_sequence: actualState.sequence,
      };
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
      for (const attempt of fixture.after.attempts) {
        const observed = await replayAfterAttempt(server, account, attempt, keys);
        result.after.push(observed);
        if (server && observed.match !== true) {
          const laterAttempts = fixture.after.attempts.slice(result.after.length);
          result.after.push(...laterAttempts.map((later) => ({
            id: later.id,
            expected: later.expected,
            status: "not_run",
            reason: observed.status === "unconfirmed" ? "prior_attempt_unconfirmed" : "prior_attempt_did_not_match",
            match: null,
          })));
          break;
        }
      }
    }
  }

  // A fixture with no transactions is a state check. The probe submits a
  // change-nothing transaction signed by every signer. The stated rule is that
  // an envelope needs the source account's LOW threshold, so it is accepted
  // only if the summed weights reach it.
  if (plan.transactions.length === 0) {
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
      match: server ? observed.status === wanted : null,
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

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const fixtures = loadChainFixtures(options.only);
  const server = connect(options.dryRun);

  const meta = { tool: "signerreach-replay/1", sdk: SDK_VERSION, horizon: HORIZON, dry_run: options.dryRun };
  meta.started = new Date().toISOString();
  if (server) {
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
  }

  const results = [];
  for (const fixture of fixtures) {
    log(`${fixture.expected.id} (${fixture.name})`);
    const result = await replay(fixture, server);
    describe(result);
    results.push(result);
  }
  meta.finished = new Date().toISOString();

  const differing = results.filter((result) => result.match === false).map((result) => result.fixture);
  const unconfirmed = results.some((result) => result.after?.some((attempt) => ["unconfirmed", "not_run"].includes(attempt.status)));
  const document = JSON.stringify({ meta, results, differing }, null, 2) + "\n";
  if (options.out) writeFileSync(options.out, document);
  else process.stdout.write(document);

  if (options.dryRun) log(`dry run: ${results.length} fixtures built and signed; nothing was sent`);
  else if (unconfirmed) log("one or more follow-up attempts were unconfirmed or not run");
  else if (differing.length === 0) log(`all ${results.length} fixtures matched`);
  else log(`DIFFERED: ${differing.join(", ")}`);
  return unconfirmed ? 2 : differing.length === 0 ? 0 : 1;
}

main().then(
  (code) => {
    process.exitCode = code;
  },
  (error) => {
    log(`replay: ${error instanceof Unusable ? error.message : "unexpected failure"}`);
    process.exitCode = 2;
  },
);
