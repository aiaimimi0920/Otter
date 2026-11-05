// 校验受审查的依赖副本；补丁身份绑定实际文件字节，而非同版本号假定。
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const hash = data => crypto.createHash("sha256").update(data).digest("hex");
const validHash = value => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);

function checkedPath(root, relative) {
  if (typeof relative !== "string" || !relative || relative.includes("\\") ||
      relative.split("/").some(p => !p || p === "." || p === ".." || p.includes(":"))) {
    throw new Error(`Invalid third-party path: ${relative}`);
  }
  let current = root;
  for (const part of relative.split("/")) {
    current = path.join(current, part);
    if (fs.lstatSync(current).isSymbolicLink()) throw new Error(`Linked source: ${relative}`);
  }
  return current;
}

function listFiles(root, prefix = "") {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const relative = prefix + entry.name;
    if (entry.isSymbolicLink()) throw new Error(`Linked source: ${relative}`);
    if (entry.isDirectory()) return listFiles(path.join(root, entry.name), relative + "/");
    if (!entry.isFile()) throw new Error(`Unsupported source entry: ${relative}`);
    return [relative];
  }).sort();
}

function verifyPackage(root, entry) {
  const manifest = JSON.parse(fs.readFileSync(checkedPath(root, entry.manifest), "utf8"));
  const schema = manifest.schemaVersion === undefined ? 1 : manifest.schemaVersion;
  if (manifest.name !== entry.name || manifest.version !== entry.version ||
      ![1, 2].includes(schema) || (schema === 2 && !Array.isArray(manifest.added)) ||
      (schema === 1 && manifest.added !== undefined) ||
      !validHash(manifest.archiveSha256) || !validHash(manifest.patchSha256) ||
      !Array.isArray(manifest.files) || !manifest.files.length || !Array.isArray(manifest.modified)) {
    throw new Error(`Invalid source manifest: ${entry.name}`);
  }
  if (hash(fs.readFileSync(checkedPath(root, entry.patch))) !== manifest.patchSha256) {
    throw new Error(`Patch hash mismatch: ${entry.name}`);
  }
  const directory = checkedPath(root, entry.directory);
  const names = new Set();
  const modified = [];
  const added = [];
  const identity = [];
  for (const file of manifest.files) {
    // A local addition has no upstream bytes. Never manufacture an upstream hash
    // or silently infer an addition from a missing/invalid field in a legacy record.
    const local = schema === 2 && file.origin === "local-added";
    if (typeof file.path !== "string" || names.has(file.path.toLowerCase()) ||
        (schema === 2 && !["upstream", "local-added"].includes(file.origin)) ||
        (schema === 1 && file.origin !== undefined) ||
        (local ? file.upstreamSha256 !== null : !validHash(file.upstreamSha256)) ||
        !validHash(file.candidateSha256)) {
      throw new Error(`Invalid source file record: ${entry.name}`);
    }
    names.add(file.path.toLowerCase());
    const actual = hash(fs.readFileSync(checkedPath(directory, file.path)));
    if (actual !== file.candidateSha256) throw new Error(`Source hash mismatch: ${entry.name}/${file.path}`);
    if (local) added.push(file.path);
    else if (actual !== file.upstreamSha256) modified.push(file.path);
    identity.push(`${file.path}\0${actual}\n`);
  }
  const expectedFiles = manifest.files.map(f => f.path).sort();
  if (JSON.stringify(listFiles(directory)) !== JSON.stringify(expectedFiles) ||
      JSON.stringify(modified.sort()) !== JSON.stringify([...manifest.modified].sort()) ||
      (schema === 2 && JSON.stringify(added.sort()) !== JSON.stringify([...manifest.added].sort()))) {
    throw new Error(`Source inventory mismatch: ${entry.name}`);
  }
  return { ...entry, sourceSha256: hash(identity.sort().join("")),
    archiveSha256: manifest.archiveSha256, patchSha256: manifest.patchSha256 };
}

function verifiedSources(root) {
  const index = JSON.parse(fs.readFileSync(checkedPath(root, "third_party/sources.json"), "utf8"));
  if (index.schemaVersion !== 1 || !Array.isArray(index.packages) || !index.packages.length) {
    throw new Error("Invalid third-party source index");
  }
  const seen = new Set();
  return index.packages.map(entry => {
    const key = `${entry.name}@${entry.version}`;
    if (seen.has(key)) throw new Error(`Duplicate source identity: ${key}`);
    seen.add(key);
    return verifyPackage(root, entry);
  });
}

function verifyResolution(root, sources, metadata) {
  if (!Array.isArray(metadata.packages) || !Array.isArray(metadata.resolve?.nodes)) {
    throw new Error("Incomplete native Cargo resolution");
  }
  for (const source of sources) {
    const matches = metadata.packages.filter(p => p.name === source.name && p.version === source.version);
    const expected = fs.realpathSync(checkedPath(root, source.directory + "/Cargo.toml"));
    if (matches.length !== 1 || matches[0].source != null ||
        !metadata.resolve.nodes.some(n => n.id === matches[0].id) ||
        fs.realpathSync(matches[0].manifest_path) !== expected) {
      throw new Error(`Patched dependency source resolution mismatch: ${source.name}`);
    }
  }
}

function resolvedSources(root) {
  const sources = verifiedSources(root);
  // 不只信任 lock 中缺少 source 的同名包：配置覆盖和另一路径也必须被发现。
  const result = spawnSync("cargo", ["metadata", "--manifest-path", path.resolve(root, "native/Cargo.toml"),
    "--format-version", "1", "--locked"],
    { cwd: root, encoding: "utf8", timeout: 180000, maxBuffer: 16 * 1024 * 1024, windowsHide: true });
  if (result.error || result.status !== 0) throw new Error(`Native Cargo resolution failed: ${result.error || result.stderr}`);
  verifyResolution(root, sources, JSON.parse(result.stdout));
  return sources;
}

if (require.main === module) process.stdout.write(JSON.stringify(resolvedSources(process.argv[2])));
module.exports = { verifiedSources, resolvedSources, verifyPackage, verifyResolution, checkedPath };
