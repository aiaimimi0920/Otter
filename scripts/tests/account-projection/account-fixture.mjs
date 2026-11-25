// Orchestrate only actual Platform services, BFF, and two Loom instances; account approval uses synthetic users.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { createServer } from "node:http";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import { nativeProjectionLifecycle } from "./application-lifecycle.mjs";
import { runWithReceipt } from "./fixture-receipt.mjs";

const platform = process.env.OTTER_TEST_PLATFORM_REPO;
const load = (path) => import(pathToFileURL(join(platform, path)).href);
const require = createRequire(join(platform, "package.json"));
const Redis = require("ioredis");
const { ProjectionRuntime, object } = await import(pathToFileURL(join(
  process.env.OTTER_TEST_LOOM_REPO, "scripts/qr-projection-smoke/runtime.ts",
)).href);
const { LoomAccountRepository } = await load("packages/account-domain/src/modules/loom-account/repository.ts");
const { LoomAccountService } = await load("packages/account-domain/src/modules/loom-account/service.ts");
const { LoomAccountError } = await load("packages/account-domain/src/modules/loom-account/model.ts");
const { LoomProjectionRepository } = await load("packages/account-domain/src/modules/loom-projection/repository.ts");
const { LoomProjectionService } = await load("packages/account-domain/src/modules/loom-projection/service.ts");
const { LoomProjectionError } = await load("packages/account-domain/src/modules/loom-projection/model.ts");
const { projectionPolicy } = await load("packages/account-domain/src/modules/loom-projection/policy.ts");
const { parseLoomAccountAuthorization } = await load("web/src/lib/loom-account-authorization.ts");
const { handleLoomAccountRequest } = await load("web/src/lib/loom-account-handlers.ts");
const { handleLoomProjectionRequest } = await load("web/src/lib/loom-projection-handlers.ts");
const redisUrl = process.env.LOOM_ACCOUNT_TEST_REDIS_URL;
assert.match(redisUrl, /^redis:\/\/127\.0\.0\.1:\d+$/);
const redis = new Redis(redisUrl, { maxRetriesPerRequest: 1 });
const service = new LoomAccountService(new LoomAccountRepository(redis));
let origin = "";
const projections = new LoomProjectionService(new LoomProjectionRepository(redis), service, () => projectionPolicy(origin, []));
const signal = AbortSignal.timeout(300_000);
const a = new ProjectionRuntime(process.env.LOOM_ACCOUNT_TEST_DAEMON, signal);
const b = new ProjectionRuntime(process.env.LOOM_ACCOUNT_TEST_DAEMON, signal);
const server = createServer(async (request, response) => {
  try {
    assert.equal(request.method, "POST");
    const chunks = [];
    let size = 0;
    for await (const chunk of request) {
      size += chunk.length;
      assert.ok(size <= 16 * 1024, "fixture body budget");
      chunks.push(chunk);
    }
    const input = new Request(origin + request.url, {
      method: "POST", headers: { "content-type": "application/json" }, body: Buffer.concat(chunks),
    });
    const action = request.url.split("/").at(-1);
    const invoke = async (_path, { body }) => {
      try {
        if (action === "projections") {
          assert.ok(!String(object(body).payload).includes("imageBase64"), "central must not receive PNGs");
          return await projections.execute(body);
        }
        if (action === "exchange") return await service.exchange(body);
        assert.ok(action === "status" || action === "revoke");
        return await service.prove(action, body);
      } catch (error) {
        if (error instanceof LoomAccountError || error instanceof LoomProjectionError) {
          throw { status: error.statusCode, code: error.code };
        }
        throw error;
      }
    };
    const result = action === "projections"
      ? await handleLoomProjectionRequest(input, invoke)
      : await handleLoomAccountRequest(input, action, invoke);
    response.writeHead(result.status, Object.fromEntries(result.headers));
    response.end(await result.text());
  } catch (error) {
    console.error("account fixture request failed", error);
    response.writeHead(500);
    response.end('{"error":{"code":"fixture_failed"}}');
  }
});
await runWithReceipt(join(process.env.OTTER_APPLICATION_EVIDENCE_ROOT, "evidence.json"), async () => {
  await new Promise((done, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", done); });
  origin = `http://127.0.0.1:${server.address().port}`;
  const started = await Promise.allSettled([a.start(), b.start()]);
  const startErrors = started.filter(result => result.status === "rejected").map(result => result.reason);
  if (startErrors.length) throw new AggregateError(startErrors, "owned daemon startup failed");
  const pending = [];
  for (const runtime of [a, b]) {
    const result = await runtime.request("/v1/account/start", { origin, deviceName: "Otter fixture" },
      { Authorization: `Bearer ${runtime.adminToken}` });
    assert.equal(result.body.status, "pending");
    const url = new URL(String(result.body.authorizationUrl));
    const grant = parseLoomAccountAuthorization(Object.fromEntries(url.searchParams));
    assert.ok(grant);
    await service.approve(grant, { userId: runtime === a ? "otter-source-account" : "otter-receiver-account", username: "Otter fixture" });
    pending.push(result.body.requestId);
  }
  await delay(5100, undefined, { signal });
  const sessions = await Promise.all([a, b].map(async (runtime, index) => {
    const result = await runtime.request("/v1/account/poll", { requestId: pending[index] },
      { Authorization: `Bearer ${runtime.adminToken}` });
    assert.equal(result.body.status, "signed_in");
    return object(result.body.session);
  }));
  assert.notEqual(sessions[0].accountId, sessions[1].accountId);
  assert.notEqual(sessions[0].deviceId, sessions[1].deviceId);
  const checks = await nativeProjectionLifecycle(a, b, object);
  return {
    status: "passed", checks, realOAuthTested: false, crossMachineNetworkingTested: false,
    publicNetworkTested: false, distinctAccountsTested: true, daemonPids: [...a.pids, ...b.pids],
  };
}, [
  async () => {
    if (!server.listening) return;
    const closed = new Promise((done, reject) => server.close(error => error ? reject(error) : done()));
    server.closeAllConnections();
    await closed;
  },
  () => redis.disconnect(),
  () => a.dispose(),
  () => b.dispose(),
]);
console.log("Otter account projection Application lifecycle passed");
