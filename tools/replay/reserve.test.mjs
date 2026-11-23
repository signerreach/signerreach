import test from "node:test";
import assert from "node:assert/strict";

import { checkReserveFigures, stroopsOf } from "./replay.mjs";

const reserve = {
  balance: 99999900000,
  selling_liabilities: 0,
  subentry_count: 2,
  num_sponsoring: 0,
  num_sponsored: 0,
  base_reserve: 5000000,
};

// What the SDK gives for a funded account that has paid its setup fees.
function account(change = {}) {
  return {
    balances: [{ asset_type: "native", balance: "9999.9999400", selling_liabilities: "0.0000000" }],
    subentry_count: 2,
    num_sponsoring: 0,
    num_sponsored: 0,
    ...change,
  };
}

test("amounts are read as whole stroops", () => {
  assert.equal(stroopsOf("0.0000001"), 1n);
  assert.equal(stroopsOf("10791.0428944"), 107910428944n);
  assert.equal(stroopsOf("922337203685.4775807"), 9223372036854775807n);
});

test("amounts in any other shape are refused", () => {
  for (const bad of ["", "1", "1.000000", "1.00000000", "01.0000000", "-1.0000000", "1e3", " 1.0000000", ".5000000"]) {
    assert.throws(() => stroopsOf(bad), /not in the expected form/, bad);
  }
});

test("a fixture without a reserve object has nothing to check", () => {
  assert.doesNotThrow(() => checkReserveFigures(account(), undefined));
});

test("a real balance above the fixture's is accepted, and so is an equal one", () => {
  assert.doesNotThrow(() => checkReserveFigures(account(), reserve));
  assert.doesNotThrow(() =>
    checkReserveFigures(
      account({ balances: [{ asset_type: "native", balance: "9999.9900000", selling_liabilities: "0.0000000" }] }),
      reserve,
    ),
  );
});

test("a real balance below the fixture's is refused", () => {
  const poorer = account({ balances: [{ asset_type: "native", balance: "9999.9899999", selling_liabilities: "0.0000000" }] });
  assert.throws(() => checkReserveFigures(poorer, reserve), /holds less than the fixture's reserve says/);
});

test("the counts and the liabilities must match exactly", () => {
  for (const change of [
    { subentry_count: 3 },
    { subentry_count: 1 },
    { num_sponsoring: 1 },
    { num_sponsored: 1 },
    { balances: [{ asset_type: "native", balance: "9999.9999400", selling_liabilities: "0.0000001" }] },
  ]) {
    assert.throws(() => checkReserveFigures(account(change), reserve), /differ from the fixture's reserve/);
  }
});

test("exactly one native balance is needed", () => {
  const native = { asset_type: "native", balance: "9999.9999400", selling_liabilities: "0.0000000" };
  assert.throws(() => checkReserveFigures(account({ balances: [] }), reserve), /exactly one native balance/);
  assert.throws(() => checkReserveFigures(account({ balances: [native, native] }), reserve), /exactly one native balance/);
  assert.throws(() => checkReserveFigures({ subentry_count: 2 }, reserve), /exactly one native balance/);
});
