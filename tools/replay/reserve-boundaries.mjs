// Records, on Stellar Testnet, the boundary cases of the balance, reserve and fee
// rules (R10 to R14 in docs/SPEC.md), and compares the checker's verdict for each
// with what the network did.
//
// For every case it:
//   1. creates a fresh account whose balance, after its setup transactions have
//      paid their fees, is an exact number of stroops;
//   2. runs `signerreach fetch` on it and `signerreach check` on the plan, as a
//      user would, before anything is submitted;
//   3. submits the plan to the network;
//   4. writes down both answers and whether they agree.
//
// Testnet only. Keys are generated in memory, used, and never printed or written.
// A case whose account does not reach its exact balance is recorded as unusable
// and is never counted as a match.
//
//   node reserve-boundaries.mjs <signerreach-binary> <output-directory>
//
// The expected results below are typed from the table in section 10 of
// docs/SPEC.md. They are not read from the checker's output.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { Horizon, Keypair, Networks, Operation, TransactionBuilder } from "@stellar/stellar-sdk";

const BINARY = process.argv[2];
const OUT = process.argv[3];
if (!BINARY || !OUT) throw new Error("usage: node reserve-boundaries.mjs <binary> <output-dir>");
mkdirSync(OUT, { recursive: true });

const HORIZON = "https://horizon-testnet.stellar.org";
const server = new Horizon.Server(HORIZON);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const log = (...parts) => console.error(...parts);
const save = (directory, name, value) => {
  mkdirSync(directory, { recursive: true });
  writeFileSync(`${directory}/${name}`, typeof value === "string" ? value : JSON.stringify(value, null, 2) + "\n");
};

const BASE_FEE = 100n; // stroops for each operation, the network's base fee (checked below)

// ----------------------------------------------------------------- helpers

const stroopsText = (amount) => {
  const whole = amount / 10_000_000n;
  const fraction = (amount % 10_000_000n).toString().padStart(7, "0");
  return `${whole}.${fraction}`;
};
const stroopsOf = (text) => {
  const match = /^(0|[1-9][0-9]{0,11})\.([0-9]{7})$/.exec(String(text));
  if (!match) throw new Error("unexpected amount form");
  return BigInt(match[1]) * 10_000_000n + BigInt(match[2]);
};
const hex = (bytes) => Buffer.from(bytes).toString("hex");

async function getJson(path) {
  const response = await fetch(`${HORIZON}${path}`);
  return { status: response.status, body: response.status === 200 ? await response.json() : null };
}

async function loadAccount(publicKey, attempts = 25) {
  for (let i = 0; i < attempts; i++) {
    try {
      return await server.loadAccount(publicKey);
    } catch {
      await sleep(1500);
    }
  }
  throw new Error("an account did not become visible");
}

// Submits a transaction and says what the network did with it. A transaction that
// fails when applied is included in a ledger (it has a record and pays its fee); a
// transaction rejected at submission is not (it should have no record).
async function submit(transaction) {
  const hash = hex(transaction.hash());
  try {
    const ok = await server.submitTransaction(transaction);
    return { hash, outcome: "accepted", ledger: ok.ledger, result_codes: null };
  } catch (error) {
    const data = error?.response?.data;
    const codes = data?.extras?.result_codes ?? null;
    if (!codes) return { hash, outcome: "unknown", http_status: error?.response?.status ?? null, result_codes: null };
    // A transaction that failed when applied is in a ledger, but Horizon may take a
    // few seconds to show it, so it is looked for longer than one that was rejected.
    const included = await lookup(hash, codes.transaction === "tx_failed" ? 12 : 4);
    return {
      hash,
      outcome: included ? "failed_in_ledger" : "rejected_at_submission",
      http_status: error.response.status,
      result_codes: codes,
      record: included,
      // Absence is cautious evidence only: it is not proof that the transaction can
      // never appear.
      absence_checks: included ? 0 : 4,
    };
  }
}

async function lookup(hash, checks) {
  for (let i = 0; i < checks; i++) {
    const found = await getJson(`/transactions/${hash}`);
    if (found.status === 200) {
      const t = found.body;
      return {
        ledger: t.ledger,
        created_at: t.created_at,
        successful: t.successful,
        fee_charged: t.fee_charged,
        max_fee: t.max_fee,
        operation_count: t.operation_count,
        result_xdr: t.result_xdr,
      };
    }
    await sleep(2000);
  }
  return null;
}

function run(args, file, directory) {
  let out = "";
  let code = 0;
  try {
    out = execFileSync(BINARY, args, { encoding: "utf8" });
  } catch (error) {
    out = (error.stdout || "") + (error.stderr || "");
    code = error.status ?? 1;
  }
  if (file) save(directory, file, out);
  return { out, code };
}

// ------------------------------------------------------------------- the run

const root = (await getJson("/")).body;
if (root.network_passphrase !== Networks.TESTNET) throw new Error("the server is not Testnet");
const ledgerNow = (await getJson(`/ledgers?order=desc&limit=1`)).body._embedded.records[0];
const baseReserve = BigInt(ledgerNow.base_reserve_in_stroops);
if (BigInt(ledgerNow.base_fee_in_stroops) !== BASE_FEE) throw new Error("the base fee is not 100 stroops; the cases assume it");
const info = {
  started_utc: new Date().toISOString(),
  horizon: HORIZON,
  horizon_version: root.horizon_version,
  core_version: root.core_version,
  protocol_version: root.current_protocol_version,
  ledger_at_start: ledgerNow.sequence,
  base_reserve_in_stroops: String(baseReserve),
  base_fee_in_stroops: String(BASE_FEE),
  checker_binary_sha256: createHash("sha256").update(readFileSync(BINARY)).digest("hex"),
  checker_version: run(["--version"], null, OUT).out.trim(),
  node: process.version,
};

const funder = Keypair.random();
const friendbot = await fetch(`https://friendbot.stellar.org?addr=${funder.publicKey()}`);
info.friendbot_status = friendbot.status;
log("friendbot", friendbot.status);

// ------------------------------------------------------------------- the cases
//
// `signers` are added in setup, so the account starts with that many sub-entries.
// `target` is the balance, in stroops, that the account must hold when the plan is
// checked and submitted. With one sub-entry the minimum balance is 3 x base reserve,
// with two it is 4 x, and with none it is 2 x.
const min = (subentries) => (2n + BigInt(subentries)) * baseReserve;

const cases = [
  {
    id: "FX-10a",
    what: "adds a signer at exactly minimum balance + base reserve + fee",
    signers: ["B"],
    target: min(1) + baseReserve + BASE_FEE,
    plan: [{ id: "add-d", ops: [["add", "D"]] }],
    expect: { verdict: "SAFE WITHIN SCOPE", network: ["accepted"], codes: [] },
  },
  {
    id: "FX-10b",
    what: "the same, one stroop less",
    signers: ["B"],
    target: min(1) + baseReserve + BASE_FEE - 1n,
    plan: [{ id: "add-d", ops: [["add", "D"]] }],
    expect: { verdict: "UNSAFE", network: ["failed_in_ledger"], codes: ["SR-RESERVE-01"], op_codes: ["op_low_reserve"] },
  },
  {
    id: "FX-11a",
    what: "[remove C, add D] at the boundary, fee 200",
    signers: ["B", "C"],
    target: min(2) + BASE_FEE * 2n,
    plan: [{ id: "swap", ops: [["remove", "C"], ["add", "D"]] }],
    expect: { verdict: "SAFE WITHIN SCOPE", network: ["accepted"], codes: [] },
  },
  {
    id: "FX-11b",
    what: "[add D, remove C] at the same balance, fee 200",
    signers: ["B", "C"],
    target: min(2) + BASE_FEE * 2n,
    plan: [{ id: "swap", ops: [["add", "D"], ["remove", "C"]] }],
    expect: { verdict: "UNSAFE", network: ["failed_in_ledger"], codes: ["SR-RESERVE-01"], op_codes: ["op_low_reserve"] },
  },
  {
    id: "FX-12",
    what: "re-weights an existing signer with no spare reserve, only the fee",
    signers: ["B"],
    target: min(1) + BASE_FEE,
    plan: [{ id: "reweight", ops: [["weight", "B", 2]] }],
    expect: { verdict: "SAFE WITHIN SCOPE", network: ["accepted"], codes: [] },
  },
  {
    id: "FX-14",
    what: "available is 99 stroops and the fee is 100",
    signers: ["B"],
    target: min(1) + BASE_FEE - 1n,
    plan: [{ id: "reweight", ops: [["weight", "B", 2]] }],
    expect: { verdict: "UNSAFE", network: ["rejected_at_submission"], codes: ["SR-RESERVE-01"], tx_code: "tx_insufficient_balance" },
  },
  {
    id: "FX-17",
    what: "a failed transaction keeps its fee, so the next one cannot pay its own",
    signers: ["B"],
    target: min(1) + BASE_FEE + 50n,
    plan: [
      { id: "add-d", ops: [["add", "D"]] },
      { id: "reweight", ops: [["weight", "B", 2]] },
    ],
    expect: {
      verdict: "UNSAFE",
      network: ["failed_in_ledger", "rejected_at_submission"],
      codes: ["SR-RESERVE-01"],
    },
  },
];

const results = [];
for (const spec of cases) {
  const directory = `${OUT}/${spec.id.toLowerCase()}`;
  const result = { id: spec.id, what: spec.what, target_balance_stroops: String(spec.target), expected: spec.expect };
  log(`\n== ${spec.id}: ${spec.what}`);
  try {
    // Fresh keys for this case. They are never written anywhere.
    const names = ["A", "B", "C", "D"];
    const keys = Object.fromEntries(names.map((name) => [name, Keypair.random()]));
    const account = keys.A;
    const publicKeys = Object.fromEntries(names.map((name) => [name, keys[name].publicKey()]));

    // 1. Create the account. Setup adds one sub-entry per signer in one transaction,
    //    which costs 100 stroops for each operation, so start with that much extra.
    const setupFee = BASE_FEE * BigInt(spec.signers.length);
    const starting = spec.target + setupFee;
    const funderAccount = await loadAccount(funder.publicKey());
    const create = new TransactionBuilder(funderAccount, { fee: "100", networkPassphrase: Networks.TESTNET })
      .addOperation(Operation.createAccount({ destination: account.publicKey(), startingBalance: stroopsText(starting) }))
      .setTimeout(60)
      .build();
    create.sign(funder);
    result.create = await submit(create);
    if (result.create.outcome !== "accepted") throw new Error("the account was not created");

    // 2. Setup: add the signers the case needs. Thresholds stay 0, so the account's
    //    own key alone authorizes every plan below.
    let current = await loadAccount(account.publicKey());
    const setup = new TransactionBuilder(current, { fee: "100", networkPassphrase: Networks.TESTNET });
    for (const name of spec.signers) {
      setup.addOperation(Operation.setOptions({ signer: { ed25519PublicKey: keys[name].publicKey(), weight: 1 } }));
    }
    const setupTx = setup.setTimeout(60).build();
    setupTx.sign(account);
    result.setup = await submit(setupTx);
    if (result.setup.outcome !== "accepted") throw new Error("setup failed");

    // 3. The balance must be exactly the target. If it is not, the case says nothing.
    current = await loadAccount(account.publicKey());
    const native = current.balances.find((entry) => entry.asset_type === "native");
    const balance = stroopsOf(native.balance);
    result.balance_at_plan_time_stroops = String(balance);
    result.subentry_count_at_plan_time = current.subentry_count;
    if (balance !== spec.target || current.subentry_count !== spec.signers.length) {
      throw new Error("the account did not reach its exact balance");
    }

    // 4. Ask the checker, exactly as a user would: fetch, then check.
    const fetched = run(["fetch", "--network", "testnet", "--account", account.publicKey()], "snapshot.json", directory);
    result.fetch_exit = fetched.code;
    const snapshot = JSON.parse(fetched.out);
    result.snapshot_reserve = snapshot.reserve;
    result.snapshot_matches_account =
      snapshot.reserve.balance === Number(balance) &&
      snapshot.reserve.subentry_count === current.subentry_count &&
      snapshot.reserve.base_reserve === Number(baseReserve);

    const planDocument = {
      schema: "signerreach.plan/1",
      transactions: spec.plan.map((tx) => ({
        id: tx.id,
        // A transaction with more than one operation states its fee, as a user would.
        ...(tx.ops.length > 1 ? { fee: Number(BASE_FEE) * tx.ops.length } : {}),
        signed_by: [account.publicKey()],
        operations: tx.ops.map(([kind, name, weight]) => ({
          op: "set_options",
          signer: { key: publicKeys[name], weight: kind === "remove" ? 0 : (weight ?? 1) },
        })),
      })),
    };
    const availability = { schema: "signerreach.availability/1", default: "available" };
    save(directory, "plan.json", planDocument);
    save(directory, "availability.json", availability);
    const args = ["check", "--snapshot", `${directory}/snapshot.json`, "--plan", `${directory}/plan.json`, "--availability", `${directory}/availability.json`];
    const text = run(args, "check.txt", directory);
    const json = run([...args, "--format", "json"], "check.json", directory);
    const report = JSON.parse(json.out);
    result.checker = {
      exit: text.code,
      verdict: report.verdict,
      codes: [...new Set(report.diagnostics.map((d) => d.code))].sort(),
      transactions: report.transactions.map((t) => t.result),
    };

    // 5. Submit the plan to the network, one transaction at a time.
    result.network = [];
    for (const tx of spec.plan) {
      current = await loadAccount(account.publicKey());
      const builder = new TransactionBuilder(current, { fee: "100", networkPassphrase: Networks.TESTNET });
      for (const [kind, name, weight] of tx.ops) {
        builder.addOperation(
          Operation.setOptions({ signer: { ed25519PublicKey: keys[name].publicKey(), weight: kind === "remove" ? 0 : (weight ?? 1) } }),
        );
      }
      const planned = builder.setTimeout(60).build();
      planned.sign(account);
      const outcome = await submit(planned);
      result.network.push({ id: tx.id, ...outcome });
      log(`   ${tx.id}: ${outcome.outcome}`, outcome.result_codes ? JSON.stringify(outcome.result_codes) : "");
    }

    // 6. What the account looks like afterwards.
    const after = await loadAccount(account.publicKey());
    const nativeAfter = after.balances.find((entry) => entry.asset_type === "native");
    result.account_after = {
      balance_stroops: String(stroopsOf(nativeAfter.balance)),
      subentry_count: after.subentry_count,
      signer_count: after.signers.length - 1,
    };
    result.accounts = { under_test: account.publicKey(), signers: { B: publicKeys.B, C: publicKeys.C, D: publicKeys.D } };

    // 7. Compare. Every part must agree with the hand-typed expectation, and the
    //    checker's verdict must agree with the network.
    const exp = spec.expect;
    const observed = result.network.map((entry) => entry.outcome);
    const opCodes = result.network.flatMap((entry) => entry.result_codes?.operations ?? []);
    const txCodes = result.network.map((entry) => entry.result_codes?.transaction ?? null);
    const checks = {
      checker_verdict_as_expected: result.checker.verdict === exp.verdict,
      checker_codes_as_expected: JSON.stringify(result.checker.codes) === JSON.stringify([...exp.codes].sort()),
      network_outcomes_as_expected: JSON.stringify(observed) === JSON.stringify(exp.network),
      operation_codes_as_expected: exp.op_codes ? JSON.stringify(opCodes.slice(0, exp.op_codes.length)) === JSON.stringify(exp.op_codes) : true,
      transaction_code_as_expected: exp.tx_code ? txCodes.includes(exp.tx_code) : true,
      fee_charged_when_included: result.network.every((entry) => entry.outcome !== "failed_in_ledger" || entry.record?.fee_charged === String(BASE_FEE * BigInt(spec.plan.find((p) => p.id === entry.id).ops.length))),
      snapshot_matches_account: result.snapshot_matches_account,
      checker_agrees_with_network:
        (result.checker.verdict === "SAFE WITHIN SCOPE") === observed.every((entry) => entry === "accepted"),
    };
    result.checks = checks;
    result.match = Object.values(checks).every(Boolean);
  } catch (error) {
    // Nothing from the error body is kept: it could echo anything.
    result.unusable = String(error?.message ?? "failed").slice(0, 200);
    result.match = null;
  }
  results.push(result);
  log(`   match: ${result.match}`);
}

info.finished_utc = new Date().toISOString();
const final = (await getJson("/")).body;
info.protocol_version_at_end = final.current_protocol_version;
info.ledger_at_end = final.history_latest_ledger;
info.funder = funder.publicKey();
const summary = {
  info,
  matched: results.filter((entry) => entry.match === true).length,
  differed: results.filter((entry) => entry.match === false).map((entry) => entry.id),
  unusable: results.filter((entry) => entry.match === null).map((entry) => entry.id),
  results,
};
save(OUT, "results.json", summary);
console.log(JSON.stringify({ matched: summary.matched, differed: summary.differed, unusable: summary.unusable }));
process.exit(summary.differed.length === 0 && summary.unusable.length === 0 ? 0 : 1);
