// Receipt-contract tests execute no compilers; write all synthetic candidates only to caller-specified temporary directories.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { create, digest, verify, validateSource } = require('../native-candidate/provenance.cjs');
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
  fs.writeFileSync(path.join(root, 'payload/otter_live_hover_hook.dll'), 'synthetic hook; not a runtime test');
  fs.writeFileSync(path.join(root, 'build.log'), 'synthetic successful build');
  write(root, 'metadata.json', {});
  for (const phase of ['before', 'after']) write(root, `source-${phase}.json`, source());
  const tools = { rustcVersion: 'rustc fixture\nhost: x86_64-pc-windows-msvc', cargoVersion: 'cargo fixture',
    environmentHashes: {}, externalCargoConfigHashes: {} };
  for (const name of ['cargo', 'rustc', 'resource', 'node', 'git']) tools[name] = { path: `C:/fixture/${name}.exe`, sha256: hash(name) };
  for (const phase of ['before', 'after']) write(root, `tools-${phase}.json`, tools);
  write(root, 'build.json', { exitCode: 0, channel: 'internal', profile: 'release', target: 'x86_64-pc-windows-msvc',
    gitDirty: true, gitCommit: 'a'.repeat(40), arguments: ['build', '--manifest-path', 'native/Cargo.toml', '--locked',
      '--offline', '--release', '-p', 'otter-native', '-p', 'otter-live-hover-hook', '--lib', '--bin', 'otter-native', '--target', 'x86_64-pc-windows-msvc'] });
  write(root, 'provenance.json', create(root));
  return root;
}
test('internal candidate binds content without granting release or runtime acceptance', () => {
  const receipt = verify(fixture());
  assert.equal(receipt.publishable, false);
  assert.equal(receipt.runtimeVerified, false);
  assert.equal(receipt.signed, false);
  assert.equal(receipt.gitDirty, true);
  assert.equal(receipt.files.length, 9);
});
test('public receipts require a fresh clean tagged build, never an internal ZIP or static approval', () => {
  const root = fixture();
  assert.throws(() => create(root, 'public'), /source binding/);
  for (const phase of ['before', 'after']) write(root, `source-${phase}.json`,
    { ...source(), internalRevision: 0, buildVersion: 'v0.2.33' });
  const build = { ...read(root, 'build.json'), channel: 'public', gitDirty: false,
    releaseSource: { tag: 'v0.2.33', repository: 'aiaimimi0920/Otter', runId: '101', attempt: '1' } };
  write(root, 'build.json', build);
  assert.equal(create(root, 'public').publishable, false);
  assert.throws(() => create(root), /source binding/);
  for (const bad of [{ ...build, gitDirty: true }, { ...build, channel: 'internal' },
    { ...build, releaseSource: { ...build.releaseSource, tag: 'v0.2.33.0' } },
    { ...build, releaseSource: { ...build.releaseSource, repository: 'other/Otter' } }]) {
    write(root, 'build.json', bad);
    assert.throws(() => create(root, 'public'), /public release source|build invocation/);
  }
  validateSource({ ...source(), internalRevision: 0, buildVersion: 'v0.2.33.0' });
});
test('streamed digests match exact bytes at empty, chunk and multi-chunk sizes', () => {
  const root = fixture();
  for (const length of [0, 65536, 65537, 131073]) {
    const bytes = Buffer.alloc(length, 0xab);
    const name = 'build.log';
    fs.writeFileSync(path.join(root, name), bytes);
    assert.deepEqual(digest(root, name, length), { path: name, bytes: length, sha256: hash(bytes) });
    if (length > 0) assert.throws(() => digest(root, name, length - 1), /Invalid candidate file/);
  }
  assert.throws(() => digest(root, 'payload'), /Invalid candidate file/);
});
for (const size of [1, 4]) {
  test(`digest refuses length ${size} after observing 3 bytes`, t => {
    const root = fixture();
    const file = path.join(root, 'build.log');
    fs.writeFileSync(file, 'abc');
    const readSync = fs.readSync;
    const fstatSync = fs.fstatSync;
    let checkedFd;
    t.mock.method(fs, 'fstatSync', fd => {
      const stat = fstatSync(fd);
      checkedFd = fd;
      fs.truncateSync(file, size);
      return stat;
    });
    const read = t.mock.method(fs, 'readSync', (...args) => readSync(...args));
    assert.throws(() => digest(root, 'build.log'), /file size changed while hashing/);
    assert.ok(read.mock.calls.every(call => call.arguments[2] === 0 && call.arguments[3] <= 3));
    assert.throws(() => fstatSync(checkedFd), { code: 'EBADF' });
  });
}
test('final staging check refuses a differently sealed but valid candidate', () => {
  const root = fixture();
  const first = verify(root);
  assert.deepEqual(verify(root, first), first);
  fs.appendFileSync(path.join(root, 'build.log'), 'replacement');
  const replacement = create(root);
  write(root, 'provenance.json', replacement);
  assert.deepEqual(verify(root), replacement);
  assert.throws(() => verify(root, first), /provenance changed during staging/);
});
for (const name of ['payload/otter-native.exe', 'payload/otter_live_hover_hook.dll', 'build.log', 'metadata.json']) {
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
