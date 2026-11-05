// 副本篡改、遗漏及路径边界必须 fail closed；由现有 security-* CI 测试入口执行。
const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const crypto = require("node:crypto");
const { verifyPackage, verifiedSources, verifyResolution } = require("../release/third-party-sources.cjs");
const hash = text => crypto.createHash("sha256").update(text).digest("hex");

function fixture() {
  const root = fs.mkdtempSync(path.join(process.env.OTTER_TEST_TEMP || os.tmpdir(), "otter-source-test-"));
  fs.mkdirSync(path.join(root, "source"));
  fs.writeFileSync(path.join(root, "source", "lib.rs"), "patched\n");
  fs.writeFileSync(path.join(root, "fix.patch"), "patch\n");
  const manifest = { name: "sample", version: "1.0.0", archiveSha256: hash("archive"),
    patchSha256: hash("patch\n"), modified: ["lib.rs"], files: [
      { path: "lib.rs", upstreamSha256: hash("original\n"), candidateSha256: hash("patched\n") },
    ] };
  const entry = { name: "sample", version: "1.0.0", directory: "source", manifest: "manifest.json", patch: "fix.patch" };
  const save = () => fs.writeFileSync(path.join(root, "manifest.json"), JSON.stringify(manifest));
  save();
  return { root, manifest, entry, save };
}

function additionFixture() {
  const f = fixture();
  Object.assign(f.manifest, { schemaVersion: 2, added: ["new.rs"] });
  f.manifest.files[0].origin = "upstream";
  fs.writeFileSync(path.join(f.root, "source", "new.rs"), "new source\n");
  f.manifest.files.push({ path: "new.rs", origin: "local-added",
    upstreamSha256: null, candidateSha256: hash("new source\n") });
  f.save();
  return f;
}

test("schema 2 explicitly inventories local additions without a forged upstream hash", () => {
  const f = additionFixture();
  const result = verifyPackage(f.root, f.entry);
  assert.equal(result.sourceSha256, hash([
    `lib.rs\0${hash("patched\n")}\n`, `new.rs\0${hash("new source\n")}\n`,
  ].sort().join("")));
  fs.writeFileSync(path.join(f.root, "source", "new.rs"), "tampered\n");
  assert.throws(() => verifyPackage(f.root, f.entry), /Source hash mismatch/);
});

test("addition origins, schema versions and exact addition inventories fail closed", () => {
  for (const mutate of [
    m => { m.schemaVersion = 3; },
    m => { m.schemaVersion = null; },
    m => { delete m.schemaVersion; },
    m => { delete m.added; },
    m => { m.added = []; },
    m => { m.added.push("new.rs"); },
    m => { m.added = ["lib.rs"]; },
    m => { m.modified.push("new.rs"); },
    m => { delete m.files[1].origin; },
    m => { m.files[1].origin = "upstream"; },
    m => { m.files[1].origin = "unknown"; },
    m => { m.files[1].upstreamSha256 = hash("fake upstream"); },
    m => { delete m.files[1].upstreamSha256; },
    m => { m.files[0].origin = "local-added"; },
    m => { m.files[1].path = "NEW.rs"; m.files.push({ ...m.files[1], path: "new.rs" }); },
  ]) {
    const f = additionFixture(); mutate(f.manifest); f.save();
    assert.throws(() => verifyPackage(f.root, f.entry));
  }
});

test("repository source copies have distinct pinned source identities", () => {
  const sources = verifiedSources(path.resolve(__dirname, "../.."));
  assert.deepEqual(sources.map(s => s.name).sort(), ["resvg", "roxmltree", "tiny-skia", "usvg"]);
  assert.equal(new Set(sources.map(s => s.sourceSha256)).size, 4);
  const workflow = fs.readFileSync(path.resolve(__dirname, "../../.github/workflows/build-otter-exe.yml"), "utf8");
  assert.ok(workflow.indexOf("node scripts/release/third-party-sources.cjs .") <
    workflow.indexOf("cargo test --locked --manifest-path native/Cargo.toml"));
  assert.match(workflow, /node scripts\/release\/third-party-sources\.cjs \.\r?\n\s+if \(\$LASTEXITCODE -ne 0\)/);
});

test("local-added dependency modules do not inherit upstream code-size exemptions", async () => {
  const { countEffectiveLines, languageForExtension } = await import("../effective-code-lines-lexer.mjs");
  const root = path.resolve(__dirname, "../..");
  for (const source of verifiedSources(root)) {
    const manifest = JSON.parse(fs.readFileSync(path.join(root, source.manifest), "utf8"));
    for (const file of manifest.files.filter(f => f.origin === "local-added")) {
      const language = languageForExtension(path.extname(file.path));
      assert.ok(language, `Local source needs a supported language checker: ${file.path}`);
      const text = fs.readFileSync(path.join(root, source.directory, file.path), "utf8");
      assert.ok(countEffectiveLines(text, language) <= 700, `Local source exceeds 700 lines: ${file.path}`);
    }
  }
});

test("source identity is deterministic and actual file changes fail closed", () => {
  const f = fixture();
  const first = verifyPackage(f.root, f.entry);
  assert.deepEqual(verifyPackage(f.root, f.entry), first);
  fs.writeFileSync(path.join(f.root, "source", "lib.rs"), "tampered\n");
  assert.throws(() => verifyPackage(f.root, f.entry), /Source hash mismatch/);
});

test("unlisted files and altered patch bytes are rejected", () => {
  const f = fixture();
  fs.writeFileSync(path.join(f.root, "source", "unexpected.rs"), "extra");
  assert.throws(() => verifyPackage(f.root, f.entry), /inventory mismatch/);
  const g = fixture();
  fs.writeFileSync(path.join(g.root, "fix.patch"), "altered");
  assert.throws(() => verifyPackage(g.root, g.entry), /Patch hash mismatch/);
});

test("duplicate, traversal and forged modified-file records are rejected", () => {
  for (const mutate of [
    m => m.files.push({ ...m.files[0] }),
    m => { m.files[0].path = "../manifest.json"; },
    m => { m.modified = []; },
    m => { m.files[0].candidateSha256 = "not-a-hash"; },
  ]) {
    const f = fixture(); mutate(f.manifest); f.save();
    assert.throws(() => verifyPackage(f.root, f.entry));
  }
});

test("Cargo resolution must bind the exact inspected directory, not only name/version", () => {
  const root = path.resolve(__dirname, "../..");
  const sources = verifiedSources(root);
  const metadata = { packages: sources.map(s => ({ name: s.name, version: s.version, id: s.name,
    source: null, manifest_path: path.join(root, s.directory, "Cargo.toml") })),
    resolve: { nodes: sources.map(s => ({ id: s.name })) } };
  verifyResolution(root, sources, metadata);
  for (const mutate of [
    m => { m.packages[0].manifest_path = path.join(root, "native/Cargo.toml"); },
    m => { m.packages[0].source = "registry+https://example.invalid"; },
    m => { m.resolve.nodes = []; },
    m => { m.packages.push(m.packages[0]); },
  ]) {
    const altered = structuredClone(metadata); mutate(altered);
    assert.throws(() => verifyResolution(root, sources, altered), /resolution mismatch/);
  }
});
