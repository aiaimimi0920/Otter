import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { runWithReceipt } from "./fixture-receipt.mjs";

function receiptPath() {
  return join(mkdtempSync(join(tmpdir(), "otter-account-receipt-")), "evidence.json");
}

test("an owned cleanup must settle before success is published", async () => {
  const path = receiptPath();
  const value = { status: "passed" };
  const receipt = runWithReceipt(path, () => value, [
    async () => {
      await Promise.resolve();
      assert.equal(existsSync(path), false);
    },
  ]);
  assert.equal(existsSync(path), false);
  await receipt;
  assert.deepEqual(JSON.parse(readFileSync(path, "utf8")), value);
  await assert.rejects(runWithReceipt(path, () => ({ status: "different" }), []), { code: "EEXIST" });
  assert.deepEqual(JSON.parse(readFileSync(path, "utf8")), value);
});

test("cleanup failures still run other owners and do not publish a success receipt", async () => {
  const path = receiptPath();
  const cleanupError = new Error("injected stop failure");
  let released = false;
  await assert.rejects(runWithReceipt(path, () => ({ status: "passed" }), [
    () => { throw cleanupError; },
    async () => { released = true; },
  ]), error => error instanceof AggregateError && error.errors[0] === cleanupError);
  assert.equal(released, true);
  assert.equal(existsSync(path), false);
});

test("multiple failures keep the primary error as well as cleanup diagnostics", async () => {
  const path = receiptPath();
  const primary = new Error("fixture failed");
  const cleanup = new Error("cleanup failed");
  await assert.rejects(runWithReceipt(path, () => { throw primary; }, [() => { throw cleanup; }]),
    error => error instanceof AggregateError && error.errors[0] === primary && error.errors[1] === cleanup);
  await assert.rejects(runWithReceipt(path, () => { throw primary; }, []), error => error === primary);
  assert.equal(existsSync(path), false);
});
