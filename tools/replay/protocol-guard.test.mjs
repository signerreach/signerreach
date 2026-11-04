import test from "node:test";
import assert from "node:assert/strict";

import { checkReplayProtocol, parseArgs } from "./replay.mjs";

test("a plain run still tests protocol 29 only", () => {
  assert.equal(parseArgs([]).networkProtocol, 29);
  assert.doesNotThrow(() => checkReplayProtocol(29, [29], 29));
  assert.throws(() => checkReplayProtocol(30, [29], 29), /asked to test protocol 29, but Testnet is on protocol 30/);
});

test("protocol 30 is tested only when it is named", () => {
  const options = parseArgs(["--network-protocol", "30"]);
  assert.equal(options.networkProtocol, 30);
  assert.doesNotThrow(() => checkReplayProtocol(30, [29], 30));
  assert.throws(() => checkReplayProtocol(29, [29], 30), /asked to test protocol 30, but Testnet is on protocol 29/);
});

test("the fixtures must stay recorded at protocol 29", () => {
  assert.throws(() => checkReplayProtocol(30, [29, 30], 30), /fixtures recorded at protocol 29/);
  assert.throws(() => checkReplayProtocol(30, [30], 30), /fixtures recorded at protocol 29/);
});

test("only known protocols are accepted as a value", () => {
  for (const value of ["28", "31", "abc", ""]) {
    assert.throws(() => parseArgs(["--network-protocol", value]), /must be one of 29, 30/);
  }
  assert.throws(() => parseArgs(["--network-protocol"]), /needs a value/);
});

test("the other options still parse beside the new one", () => {
  const options = parseArgs(["--dry-run", "--only", "fx-01", "--out", "results.json", "--network-protocol", "30"]);
  assert.deepEqual(options, { dryRun: true, only: "fx-01", out: "results.json", networkProtocol: 30 });
});
