// Reproduces a suspected false-safe in SignerReach: the checker never looks at an
// account's balance, so a plan that adds a signer to an account that cannot afford
// the extra 0.5 XLM reserve may be reported SAFE and then fail on the network.
//
// Testnet only. Throwaway keys are generated in memory and never written out.
//
//   node reserve-gap.mjs <path-to-signerreach-binary> <output-directory>
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import {
  Horizon, Keypair, Networks, Operation, TransactionBuilder,
} from "@stellar/stellar-sdk";

const BIN = process.argv[2];
const OUT = process.argv[3];
if (!BIN || !OUT) throw new Error("usage: node reserve-gap.mjs <binary> <output-dir>");
mkdirSync(OUT, { recursive: true });

const HORIZON = "https://horizon-testnet.stellar.org";
const server = new Horizon.Server(HORIZON);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = { started_utc: new Date().toISOString(), horizon: HORIZON, steps: [] };
const step = (name, data) => { log.steps.push({ name, at: new Date().toISOString(), ...data }); console.log(name, JSON.stringify(data)); };

const funder = Keypair.random();   // pays for the created account
const tight = Keypair.random();    // the account under test
const added = Keypair.random();    // the signer the plan tries to add

async function loadRetry(pub) {
  for (let i = 0; i < 20; i++) {
    try { return await server.loadAccount(pub); } catch { await sleep(1500); }
  }
  throw new Error("account not visible: " + pub);
}

// 1. Fund the funder with Friendbot.
const fb = await fetch(`https://friendbot.stellar.org?addr=${funder.publicKey()}`);
step("friendbot", { status: fb.status });

// 2. Create the account under test with a balance a little above its minimum.
//    Minimum for an account with no subentries is 2 x 0.5 = 1.0 XLM. Adding one
//    signer raises it to 1.5 XLM. 1.0010 XLM can pay a fee but cannot afford that.
const fAcc = await loadRetry(funder.publicKey());
const create = new TransactionBuilder(fAcc, { fee: "100", networkPassphrase: Networks.TESTNET })
  .addOperation(Operation.createAccount({ destination: tight.publicKey(), startingBalance: "1.0010000" }))
  .setTimeout(60).build();
create.sign(funder);
const created = await server.submitTransaction(create);
step("create-account", { hash: created.hash, ledger: created.ledger, account: tight.publicKey(), starting_balance: "1.0010000" });

// 3. What the network says about the account.
const acc = await loadRetry(tight.publicKey());
const native = acc.balances.find((b) => b.asset_type === "native");
step("account-state", {
  balance: native.balance, selling_liabilities: native.selling_liabilities,
  subentry_count: acc.subentry_count, num_sponsoring: acc.num_sponsoring, num_sponsored: acc.num_sponsored,
});

// 4. Ask SignerReach, exactly as a user would: fetch, then check an "add one signer" plan.
const run = (args, file) => {
  let out = "", code = 0;
  try { out = execFileSync(BIN, args, { encoding: "utf8" }); }
  catch (e) { out = (e.stdout || "") + (e.stderr || ""); code = e.status ?? 1; }
  if (file) writeFileSync(`${OUT}/${file}`, out);
  return { out, code };
};
const snap = run(["fetch", "--network", "testnet", "--account", tight.publicKey()], "snapshot.json");
step("signerreach-fetch", { exit: snap.code });

const plan = {
  schema: "signerreach.plan/1",
  transactions: [{
    id: "add-signer",
    signed_by: [tight.publicKey()],
    operations: [{ op: "set_options", signer: { key: added.publicKey(), weight: 1 } }],
  }],
};
const avail = { schema: "signerreach.availability/1", default: "available" };
writeFileSync(`${OUT}/plan.json`, JSON.stringify(plan, null, 2) + "\n");
writeFileSync(`${OUT}/availability.json`, JSON.stringify(avail, null, 2) + "\n");
const chk = run(["check", "--snapshot", `${OUT}/snapshot.json`, "--plan", `${OUT}/plan.json`,
  "--availability", `${OUT}/availability.json`], "check.txt");
run(["check", "--snapshot", `${OUT}/snapshot.json`, "--plan", `${OUT}/plan.json`,
  "--availability", `${OUT}/availability.json`, "--format", "json"], "check.json");
const verdict = (chk.out.match(/^VERDICT: (.*)$/m) || [])[1];
step("signerreach-check", { exit: chk.code, verdict });

// 5. Submit the same change to the network, signed by the account itself.
const tAcc = await loadRetry(tight.publicKey());
const addTx = new TransactionBuilder(tAcc, { fee: "100", networkPassphrase: Networks.TESTNET })
  .addOperation(Operation.setOptions({ signer: { ed25519PublicKey: added.publicKey(), weight: 1 } }))
  .setTimeout(60).build();
addTx.sign(tight);
const computedHash = addTx.hash().toString("hex");
try {
  const ok = await server.submitTransaction(addTx);
  step("network-submit", { outcome: "accepted", hash: ok.hash, ledger: ok.ledger });
} catch (e) {
  const d = e?.response?.data;
  step("network-submit", {
    outcome: "failed", computed_hash: computedHash,
    http_status: e?.response?.status,
    result_codes: d?.extras?.result_codes ?? null,
    message: d?.title ?? String(e),
  });
}

log.finished_utc = new Date().toISOString();
log.accounts = { under_test: tight.publicKey(), funder: funder.publicKey(), signer_added_by_plan: added.publicKey() };
writeFileSync(`${OUT}/run-log.json`, JSON.stringify(log, null, 2) + "\n");
console.log("done:", OUT);
