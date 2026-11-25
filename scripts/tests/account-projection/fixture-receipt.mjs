// Create success receipts exclusively only after both business work and this process's resource cleanup succeed.
import { writeFileSync } from "node:fs";

export async function runWithReceipt(path, run, cleanup) {
  let value;
  const failures = [];
  try {
    value = await run();
  } catch (error) {
    failures.push(error);
  }
  // Even if one owner throws synchronously, all other owners still complete cleanup.
  const results = await Promise.allSettled(cleanup.map(async close => close()));
  const primaryCount = failures.length;
  for (const result of results) {
    if (result.status === "rejected") failures.push(result.reason);
  }
  if (failures.length > primaryCount) {
    throw new AggregateError(failures, "owned account fixture cleanup failed");
  }
  if (failures.length) throw failures[0];
  writeFileSync(path, JSON.stringify(value, null, 2) + "\n", { encoding: "utf8", flag: "wx" });
}
