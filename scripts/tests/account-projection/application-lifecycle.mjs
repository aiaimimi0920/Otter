// 每步重新启动真实 Application，核验落盘恢复及接收像素，不复制生产投射逻辑。
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { closeSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = process.env.OTTER_APPLICATION_EVIDENCE_ROOT;
const driver = "desktop::application::workspace_checkpoint::tests::account_joint::actual_account_application_create_import_update_reopen_and_stop";
const launcher = fileURLToPath(new URL("./isolated.py", import.meta.url));

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, process.env.OTTER_TEST_LAN_HOST, resolve);
  });
  const address = server.address();
  assert.ok(address && typeof address === "object");
  await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  return address.port;
}

export async function nativeProjectionLifecycle(a, b, object) {
  let serial = 0;
  async function native(runtime, body) {
    const label = `application-${++serial}-${body.action}`;
    const spec = join(root, `${label}-spec.json`);
    const output = join(root, `${label}-result.json`);
    writeFileSync(spec, JSON.stringify(body), { encoding: "utf8", flag: "wx" });
    const fd = openSync(join(root, `${label}-launcher.log`), "wx");
    try {
      const child = spawn(process.env.OTTER_TEST_PYTHON, ["-B", launcher, label, driver, "--exact", "--ignored"], {
        cwd: root, windowsHide: true, stdio: ["ignore", fd, fd],
        env: { ...process.env, OTTER_PROJECTION_ACCOUNT_SPEC: spec, OTTER_PROJECTION_ACCOUNT_RESULT: output,
          LOOM_MANIFEST_PATH: join(runtime.root, "capabilities", "loom.json"),
          OTTER_PROJECTION_ACCOUNT_PROFILE: join(root, runtime === a ? "source-profile" : "receiver-profile") },
      });
      const status = await new Promise((resolve, reject) => { child.once("error", reject); child.once("exit", resolve); });
      assert.equal(status, 0, `${label} failed; see native log`);
    } finally {
      closeSync(fd);
    }
    return object(JSON.parse(readFileSync(output, "utf8")));
  }
  for (const runtime of [a, b]) {
    const result = await runtime.request("/v1/otter/connections/lan",
      { enabled: true, host: process.env.OTTER_TEST_LAN_HOST, port: await freePort() },
      { Authorization: `Bearer ${runtime.adminToken}` }, "PUT");
    assert.equal(result.status, 200, JSON.stringify(result.body));
  }
  const created = await native(a, { action: "create", pixel: 17 });
  const envelope = object(created.envelope);
  assert.equal(envelope.protocol, "neuro.qr-projection.v2");
  assert.equal(created.revision, 1);
  const accepted = await native(b, { action: "import", envelope });
  assert.equal(accepted.digest, created.digest);
  assert.deepEqual(object(accepted.pixels).original, [17, 83, 211, 255]);
  assert.deepEqual(object(accepted.pixels).center, [17, 83, 211, 255]);
  assert.equal(accepted.documentCount, 2);
  for (const revision of [2, 3]) {
    if (revision === 3) {
      await Promise.all([a.stop(), b.stop()]);
      await Promise.all([a.start(), b.start()]);
    }
    const updated = await native(a, { action: "update", pixel: 17 + revision, revision });
    const received = await native(b, { action: "read", revision });
    assert.equal(received.revision, revision);
    assert.equal(received.digest, updated.digest);
    assert.deepEqual(object(received.pixels).original, [17 + revision, 83, 211, 255]);
    assert.deepEqual(object(received.pixels).center, [17 + revision, 83, 211, 255]);
    assert.deepEqual(object(received.pixels).corner, [0, 0, 0, 0]);
    assert.equal(received.unit, accepted.unit);
    assert.equal(received.documentCount, 2);
  }
  const stopped = await native(b, { action: "stop" });
  assert.equal(stopped.stopped, true);
  assert.deepEqual(object(stopped.pixels).original, [20, 83, 211, 255]);
  assert.equal((await native(a, { action: "stop" })).stopped, true);
  return ["application_source_create_checkpoint_reopen", "application_receiver_import_checkpoint_reopen",
    "application_two_png_updates_exact_pixels", "two_loom_restart_same_profiles", "application_unlink_preserves_final_pixels"];
}
