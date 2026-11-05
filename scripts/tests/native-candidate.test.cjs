// 收据合同测试不执行编译器；所有合成候选只写入调用者指定的临时目录。
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { create, verify, validateSource } = require('../native-candidate/provenance.cjs');
const { hash, assertUnchanged } = require('../native-candidate/source.cjs');
const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
fs.mkdirSync(base, { recursive: true });
const write = (root, name, value) => fs.writeFileSync(path.join(root, name), JSON.stringify(value) + '\n', 'utf8');
const read = (root, name) => JSON.parse(fs.readFileSync(path.join(root, name), 'utf8'));
function source() {
  const files = [{ path: 'native/src/main.rs', bytes: 10, sha256: hash('fixture') }];
  return { schemaVersion: 1, kind: 'otter-native-build-inputs', files, sourceSha256: hash(JSON.stringify(files)),
    productVersion: '0.2.33', internalRevision: 231, buildVersion: 'v0.2.33.231' };
}
function fixture() {
  const root = fs.mkdtempSync(path.join(base, 'contract-'));
  fs.mkdirSync(path.join(root, 'payload'));
  fs.writeFileSync(path.join(root, 'payload/otter-native.exe'), 'synthetic binary; not a runtime test');
  fs.writeFileSync(path.join(root, 'build.log'), 'synthetic successful build');
  write(root, 'metadata.json', {});
  for (const phase of ['before', 'after']) write(root, `source-${phase}.json`, source());
  const tools = { rustcVersion: 'rustc fixture\nhost: x86_64-pc-windows-msvc', cargoVersion: 'cargo fixture',
    environmentHashes: {}, externalCargoConfigHashes: {} };
  for (const name of ['cargo', 'rustc', 'resource', 'node', 'git']) tools[name] = { path: `C:/fixture/${name}.exe`, sha256: hash(name) };
  for (const phase of ['before', 'after']) write(root, `tools-${phase}.json`, tools);
  write(root, 'build.json', { exitCode: 0, channel: 'internal', profile: 'release', target: 'x86_64-pc-windows-msvc',
    gitDirty: true, gitCommit: 'a'.repeat(40), arguments: ['build', '--manifest-path', 'native/Cargo.toml', '--locked',
      '--offline', '--release', '--bin', 'otter-native', '--target', 'x86_64-pc-windows-msvc'] });
  write(root, 'provenance.json', create(root));
  return root;
}
test('internal candidate binds content without granting release or runtime acceptance', () => {
  const receipt = verify(fixture());
  assert.equal(receipt.publishable, false);
  assert.equal(receipt.runtimeVerified, false);
  assert.equal(receipt.signed, false);
  assert.equal(receipt.gitDirty, true);
  assert.equal(receipt.files.length, 8);
});
for (const name of ['payload/otter-native.exe', 'build.log', 'metadata.json']) {
  test(`tampering is rejected: ${name}`, () => {
    const root = fixture();
    fs.appendFileSync(path.join(root, name), 'modified');
    assert.throws(() => verify(root), /mismatch/);
  });
}
test('source changes and malformed source bindings are rejected', () => {
  const before = source();
  assert.throws(() => assertUnchanged(before, { ...before, internalRevision: 232 }), /changed/);
  for (const invalid of ['../escape', '/absolute', 'C:/escape', 'native\\file', 'native/./file', 'native/\0file', 10]) {
    const value = source();
    value.files[0].path = invalid;
    value.sourceSha256 = hash(JSON.stringify(value.files));
    assert.throws(() => validateSource(value), /source entry/);
  }
  const duplicate = source();
  duplicate.files.push({ ...duplicate.files[0], path: 'NATIVE/SRC/MAIN.RS' });
  duplicate.sourceSha256 = hash(JSON.stringify(duplicate.files));
  assert.throws(() => validateSource(duplicate), /source entry/);
  assert.throws(() => validateSource({ ...source(), sourceSha256: '0'.repeat(64) }), /source binding/);
});
test('source and tool snapshots must agree before and after', () => {
  for (const name of ['source-after.json', 'tools-after.json']) {
    const root = fixture();
    const value = read(root, name);
    value.extra = 'changed';
    write(root, name, value);
    assert.throws(() => create(root), /changed/);
  }
});
test('matching empty tool snapshots are not accepted', () => {
  const root = fixture();
  for (const phase of ['before', 'after']) write(root, `tools-${phase}.json`, {});
  assert.throws(() => create(root), /tool binding/);
});
test('failed candidate and fabricated acceptance claims are rejected', () => {
  const root = fixture();
  const receipt = read(root, 'provenance.json');
  receipt.publishable = true;
  write(root, 'provenance.json', receipt);
  assert.throws(() => verify(root), /mismatch/);
  fs.writeFileSync(path.join(root, 'FAILED.txt'), 'failure');
  assert.throws(() => create(root), /Failed native candidate/);
});
test('build command cannot drop offline/locked or target another product', () => {
  for (const change of [b => b.arguments.splice(4, 1), b => b.arguments.push('--features=other'),
    b => b.channel = 'public', b => b.exitCode = 1, b => b.profile = 'debug', b => b.gitDirty = 'false']) {
    const root = fixture();
    const build = read(root, 'build.json');
    change(build);
    write(root, 'build.json', build);
    assert.throws(() => create(root), /build invocation/);
  }
});
