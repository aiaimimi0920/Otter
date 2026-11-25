// 成功回执只在业务和本进程的资源清理均成功后独占创建。
import { writeFileSync } from "node:fs";

export async function runWithReceipt(path, run, cleanup) {
  let value;
  const failures = [];
  try {
    value = await run();
  } catch (error) {
    failures.push(error);
  }
  // 即使单个 owner 同步抛错，其他 owner 仍会完成清理。
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
