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
// not be completed (network trouble, unsupported fixture, bad usage).

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
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
    fixtures.push({
      name,
      expected,
      snapshot: readJson(join(directory, "snapshot.json")),
      plan: readJson(join(directory, "plan.json")),
    });
  }
  if (fixtures.length === 0) throw new Unusable("no chain fixtures matched");
  return fixtures;
}

// One fresh keypair for every address the fixture mentions. The fixture's own
// account becomes the account that is built and tested.
function keypairsFor({ snapshot, plan }) {
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

  const checks = [...result.transactions, ...(result.probe ? [result.probe] : [])];
  result.match = server ? checks.every((check) => check.match) : null;
  return result;
}

function describe(result) {
  const checks = [...result.transactions, ...(result.probe ? [{ id: "probe", ...result.probe }] : [])];
  for (const check of checks) {
    const where = check.ledger ? ` ledger ${check.ledger}` : "";
    const mark = check.match === null ? "-" : check.match ? "ok" : "DIFFERS";
    log(`  ${check.id}: expected ${check.expected}, observed ${check.status}${where} [${mark}]`);
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
  const document = JSON.stringify({ meta, results, differing }, null, 2) + "\n";
  if (options.out) writeFileSync(options.out, document);
  else process.stdout.write(document);

  if (options.dryRun) log(`dry run: ${results.length} fixtures built and signed; nothing was sent`);
  else if (differing.length === 0) log(`all ${results.length} fixtures matched`);
  else log(`DIFFERED: ${differing.join(", ")}`);
  return differing.length === 0 ? 0 : 1;
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
