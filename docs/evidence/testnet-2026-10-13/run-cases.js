// THROWAWAY experiment for SignerReach Phase 1 (not project code, not for a repo).
// Runs the proof-obligation cases on Stellar Testnet with freshly generated throwaway keys.
// Secret keys stay in memory only. Public keys, tx hashes and result codes are written to results.json.
const fs = require("fs");
const {
  Horizon, Keypair, TransactionBuilder, Operation, Networks, BASE_FEE,
} = require("@stellar/stellar-sdk");

const HORIZON = "https://horizon-testnet.stellar.org";
const server = new Horizon.Server(HORIZON);
const NET = Networks.TESTNET;
const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function fund(kp) {
  for (let i = 0; i < 4; i++) {
    const r = await fetch(`https://friendbot.stellar.org/?addr=${kp.publicKey()}`);
    if (r.ok) return;
    await sleep(2000);
  }
  throw new Error("friendbot failed for " + kp.publicKey());
}

// Submit a transaction. sourceKp = transaction source, ops = array of Operation XDR, signers = keypairs.
async function submit(label, sourceKp, ops, signers) {
  const acct = await server.loadAccount(sourceKp.publicKey());
  const b = new TransactionBuilder(acct, { fee: BASE_FEE, networkPassphrase: NET });
  ops.forEach((o) => b.addOperation(o));
  const tx = b.setTimeout(120).build();
  signers.forEach((k) => tx.sign(k));
  try {
    const r = await server.submitTransaction(tx);
    return { label, ok: true, hash: r.hash, ledger: r.ledger, sigs: signers.length, nops: ops.length };
  } catch (e) {
    const d = e.response && e.response.data;
    const rc = d && d.extras && d.extras.result_codes;
    return { label, ok: false, hash: tx.hash().toString("hex"), codes: rc || String(e.message).slice(0, 200), sigs: signers.length, nops: ops.length };
  }
}

async function state(kp) {
  const a = await server.loadAccount(kp.publicKey());
  return {
    thresholds: a.thresholds,
    signers: a.signers.map((s) => `${s.key.slice(0, 5)}..${s.key.slice(-4)}:${s.weight}`),
  };
}

function record(caseId, desc, expectation, step, extra = {}) {
  const row = { case: caseId, desc, expected: expectation, observed: step, ...extra };
  results.push(row);
  const obs = step.ok ? `SUCCESS ledger ${step.ledger}` : `FAILED ${JSON.stringify(step.codes)}`;
  console.log(`[${caseId}] ${desc}\n    expected: ${expectation}\n    observed: ${obs}`);
}

const setOpts = (o) => Operation.setOptions(o);
const signerOp = (kp, w, source) => Operation.setOptions({ signer: { ed25519PublicKey: kp.publicKey(), weight: w }, ...(source ? { source } : {}) });

async function setup(A, { master, thresholds, signers }) {
  // Initial account has master weight 1 and thresholds 0, so one signature from A suffices for setup.
  const ops = [];
  ops.push(setOpts({ masterWeight: master, lowThreshold: thresholds[0], medThreshold: thresholds[1], highThreshold: thresholds[2] }));
  signers.forEach(([kp, w]) => ops.push(signerOp(kp, w)));
  const r = await submit("setup", A, ops, [A]);
  if (!r.ok) throw new Error("setup failed " + JSON.stringify(r));
  return r;
}

(async () => {
  const meta = { started: new Date().toISOString(), horizon: HORIZON, sdk: JSON.parse(fs.readFileSync(__dirname + "/node_modules/@stellar/stellar-sdk/package.json", "utf8")).version };
  const root = await (await fetch(HORIZON)).json();
  meta.protocol = root.current_protocol_version;
  meta.core = root.core_version;
  meta.ledgerAtStart = root.history_latest_ledger;
  console.log("Testnet", meta);

  // ---------- Case 1: unsafe change (raise HIGH above reachable weight) ----------
  {
    const A = Keypair.random(), B = Keypair.random();
    await fund(A);
    await setup(A, { master: 1, thresholds: [1, 1, 1], signers: [[B, 1]] });
    const s0 = await state(A);
    const plan = await submit("plan", A, [setOpts({ highThreshold: 2 })], [A]);
    record("1a", "plan: SetOptions(high=2) signed by A (A:1,B:1; thr 1/1/1)", "SUCCESS (A meets HIGH 1 in pre-state)", plan, { account: A.publicKey(), before: s0 });
    const s1 = await state(A);
    const fixA = await submit("post-A-alone", A, [setOpts({ highThreshold: 0 })], [A]);
    record("1b", "after plan: SetOptions(high=0) signed by A alone (B declared lost)", "FAILED op_bad_auth (A weight 1 < HIGH 2)", fixA, { after: s1 });
    const fixAB = await submit("control-A+B", A, [setOpts({ highThreshold: 0 })], [A, B]);
    record("1c", "control: same op signed by A+B", "SUCCESS (weight 2 meets HIGH 2)", fixAB);
  }

  // ---------- Case 2: safe rotation in ONE transaction ----------
  {
    const A = Keypair.random(), B = Keypair.random(), C = Keypair.random(), D = Keypair.random();
    await fund(A);
    await setup(A, { master: 1, thresholds: [1, 2, 2], signers: [[B, 1], [C, 1]] });
    const s0 = await state(A);
    const plan = await submit("plan", A, [signerOp(D, 1), signerOp(C, 0)], [A, B]);
    record("2a", "one tx [add D:1, remove C] signed A+B (thr 1/2/2)", "SUCCESS (both ops authorized vs pre-state A+B=2)", plan, { account: A.publicKey(), before: s0 });
    const s1 = await state(A);
    const viaAD = await submit("post-A+D", A, [setOpts({ highThreshold: 2 })], [A, D]);
    record("2b", "after plan: SetOptions(high=2) signed by A+D", "SUCCESS (A1+D1=2 meets HIGH 2)", viaAD, { after: s1 });
    const viaAC = await submit("post-A+C", A, [setOpts({ highThreshold: 2 })], [A, C]);
    record("2c", "after plan: same op signed by A+C (C was removed)", "FAILED (C no longer a signer; A alone 1 < 2; C signature unused)", viaAC);
  }

  // ---------- Case 3: structural deadlock (master weight to 0) ----------
  {
    const A = Keypair.random(), B = Keypair.random();
    await fund(A);
    await setup(A, { master: 1, thresholds: [1, 1, 2], signers: [[B, 1]] });
    const s0 = await state(A);
    const plan = await submit("plan", A, [setOpts({ masterWeight: 0 })], [A, B]);
    record("3a", "SetOptions(master weight 0) signed A+B (A:1,B:1; thr 1/1/2)", "SUCCESS (A+B=2 meets HIGH 2 in pre-state)", plan, { account: A.publicKey(), before: s0 });
    const s1 = await state(A);
    const fixB = await submit("post-B-alone", A, [setOpts({ highThreshold: 0 })], [B]);
    record("3b", "after plan: SetOptions(high=0) signed by B (B available)", "FAILED op_bad_auth (only B:1 left, HIGH 2: deadlock even with B available)", fixB, { after: s1 });
  }

  // ---------- Case 5a: multi-operation boundary, single transaction ----------
  {
    const A = Keypair.random(), B = Keypair.random();
    await fund(A);
    await setup(A, { master: 1, thresholds: [1, 2, 2], signers: [[B, 1]] });
    const plan = await submit("plan", A, [signerOp(B, 0), setOpts({ masterWeight: 2 })], [A, B]);
    record("5a", "ONE tx [remove B, master weight 2] signed A+B (thr 1/2/2)", "SUCCESS (both ops authorized vs pre-state A+B=2)", plan, { account: A.publicKey() });
    record("5a-state", "state after 5a", "signers {A:2}", { ok: true, ledger: plan.ledger }, { after: await state(A) });
  }

  // ---------- Case 5b: same changes as TWO transactions ----------
  {
    const A = Keypair.random(), B = Keypair.random();
    await fund(A);
    await setup(A, { master: 1, thresholds: [1, 2, 2], signers: [[B, 1]] });
    const t1 = await submit("tx1", A, [signerOp(B, 0)], [A, B]);
    record("5b-1", "TWO txs: tx1 [remove B] signed A+B", "SUCCESS", t1, { account: A.publicKey() });
    const t2 = await submit("tx2", A, [setOpts({ masterWeight: 2 })], [A]);
    record("5b-2", "TWO txs: tx2 [master weight 2] signed A alone (B gone; A:1 < HIGH 2)", "FAILED op_bad_auth (differs from 5a)", t2, { after: await state(A) });
  }

  // ---------- Case 5c: extra-signature trap ----------
  {
    const A = Keypair.random(), B = Keypair.random();
    await fund(A);
    const plan = await submit("plan", A, [
      setOpts({ masterWeight: 1, lowThreshold: 2, medThreshold: 2, highThreshold: 2, signer: { ed25519PublicKey: B.publicKey(), weight: 1 } }),
      setOpts({ homeDomain: "example.org" }),
    ], [A, B]);
    record("5c", "ONE tx [add B:1 + thresholds 2/2/2, setHomeDomain] signed A+B on a default account", "FAILED tx_bad_auth_extra (B's signature unused: pre-state thresholds 0)", plan, { account: A.publicKey() });
  }

  // ---------- Case 7: LOW threshold gates the envelope; recovery via second source (C2) ----------
  {
    const A = Keypair.random(), X = Keypair.random();
    await fund(A); await fund(X);
    const lock = await submit("lock", A, [setOpts({ lowThreshold: 5 })], [A]);
    record("7a", "SetOptions(low=5) on A with only A:1", "SUCCESS (pre-state thresholds 0)", lock, { account: A.publicKey() });
    const stuck = await submit("stuck", A, [setOpts({ lowThreshold: 0 })], [A]);
    record("7b", "try to reset low=0 with A as tx source", "FAILED tx_bad_auth (envelope needs LOW 5; A:1)", stuck);
    const rec = await submit("recover", X, [setOpts({ source: A.publicKey(), lowThreshold: 0 })], [X, A]);
    record("7c", "recover: tx source X (funded), op source A, signed X+A", "SUCCESS (op needs HIGH 0 on A; envelope LOW 0 on X)", rec, { after: await state(A) });
  }

  meta.finished = new Date().toISOString();
  fs.writeFileSync(__dirname + "/results.json", JSON.stringify({ meta, results }, null, 2));
  console.log("\nwrote results.json with", results.length, "rows");
})().catch((e) => { console.error("FATAL", e); process.exit(1); });
