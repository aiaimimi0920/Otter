// Exact upstream pin, checkout tampering and actual source/license inventory contracts.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const childProcess = require('node:child_process');
const rxing = require('../native-candidate/rxing-source.cjs');
const { snapshot, hash } = require('../native-candidate/source.cjs');
const { collect } = require('../native-candidate/licenses.cjs');
const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
fs.mkdirSync(base, { recursive: true });
const repo = path.resolve(__dirname, '../..');

function fixture(t) {
  const root = fs.mkdtempSync(path.join(base, 'rxing-source-'));
  const cargoHome = path.join(root, 'cargo');
  const checkout = path.join(cargoHome, 'git/checkouts/rxing-aabbccdd', rxing.REVISION.slice(0, 7));
  fs.mkdirSync(path.join(checkout, 'src'), { recursive: true });
  fs.mkdirSync(path.join(root, 'native'));
  fs.writeFileSync(path.join(root, 'native/Cargo.lock'),
    `[[package]]\nname = "${rxing.NAME}"\nversion = "${rxing.VERSION}"\nsource = "${rxing.SOURCE}"\n`);
  for (const [name, text] of Object.entries({ 'Cargo.toml': '[package]\nname = "rxing"\n',
    LICENSE: 'synthetic license, not upstream legal material', 'README.md': 'fixture', 'src/lib.rs': 'pub fn fixture() {}\n' })) {
    fs.writeFileSync(path.join(checkout, name), text);
  }
  const oldHome = process.env.CARGO_HOME;
  process.env.CARGO_HOME = cargoHome;
  t.after(() => {
    if (oldHome === undefined) delete process.env.CARGO_HOME;
    else process.env.CARGO_HOME = oldHome;
  });
  const pkg = { name: rxing.NAME, version: rxing.VERSION, source: rxing.SOURCE,
    id: 'fixture-rxing', license: 'Apache-2.0', manifest_path: path.join(checkout, 'Cargo.toml') };
  const metadata = { packages: [pkg], resolve: { nodes: [{ id: pkg.id, features: ['decoders', 'encoding_rs', 'qrcode'] }] } };
  const blobs = ['Cargo.toml', 'LICENSE', 'README.md', 'src/lib.rs'].map(name => {
    const bytes = fs.readFileSync(path.join(checkout, name));
    return `${name.endsWith('.rs') ? '100755' : '100644'} blob ${
      crypto.createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex')}\t${name}\0`;
  }).join('');
  const state = { head: rxing.REVISION, dirty: '', blobs };
  const git = t.mock.method(childProcess, 'execFileSync', (command, args, options) => {
    assert.equal(command, 'git');
    assert.deepEqual(args.slice(0, 4), ['--no-optional-locks', '--no-replace-objects', '-C', checkout]);
    assert.equal(options.windowsHide, true);
    assert.equal(options.timeout, 30000);
    if (args[4] === 'rev-parse') return state.head;
    if (args[4] === 'ls-tree') return state.blobs;
    assert.deepEqual(args.slice(4), ['status', '--porcelain', '--untracked-files=all',
      '--ignored=matching', '--', '.', ':(exclude).cargo-ok']);
    return state.dirty;
  });
  return { root, checkout, pkg, metadata, state, git };
}

test('only the reviewed exact upstream identity and lock binding are admitted', t => {
  const f = fixture(t);
  assert.equal(rxing.directory(f.root, f.pkg), f.checkout);
  for (const changes of [{ name: 'other' }, { version: '0.9.3' }, { license: 'MIT' }, { source: null },
    { source: rxing.SOURCE.replace('/rxing?', '/other?') },
    { source: rxing.SOURCE.replaceAll(rxing.REVISION, 'a'.repeat(40)) },
    { source: `registry+https://github.com/rust-lang/crates.io-index` },
    { manifest_path: path.join(f.root, 'native/Cargo.toml') },
    { manifest_path: path.join(f.checkout, 'src/Cargo.toml') }]) {
    assert.throws(() => rxing.directory(f.root, { ...f.pkg, ...changes }), /identity|resolution mismatch/);
  }
  fs.writeFileSync(path.join(f.root, 'native/Cargo.lock'), '[[package]]\nname = "rxing"\n');
  assert.throws(() => rxing.directory(f.root, f.pkg), /lockfile/);
});

test('wrong HEAD and tracked, untracked or ignored checkout tampering fail closed', t => {
  const f = fixture(t);
  f.state.head = 'a'.repeat(40);
  assert.throws(() => rxing.directory(f.root, f.pkg), /pinned commit/);
  f.state.head = rxing.REVISION;
  for (const dirty of [' M src/lib.rs', '?? build.rs', '!! src/injected.rs', ' M LICENSE']) {
    f.state.dirty = dirty;
    assert.throws(() => rxing.directory(f.root, f.pkg), /dirty/);
  }
});

test('source inventory is byte-bound and refuses duplicate or missing dependency identities', t => {
  const f = fixture(t);
  const before = rxing.sourceFiles(f.root, f.metadata);
  assert.deepEqual(before.map(row => row.path), ['Cargo.toml', 'LICENSE', 'README.md', 'src/lib.rs']
    .map(name => `third-party-git/rxing/${name}`));
  assert.equal(before.find(row => row.path.endsWith('/src/lib.rs')).sha256, hash('pub fn fixture() {}\n'));
  assert.throws(() => rxing.sourceFiles(f.root, { packages: [] }), /Missing/);
  assert.throws(() => rxing.sourceFiles(f.root, { packages: [f.pkg, f.pkg] }), /Duplicate/);
  // Index flags or filters cannot hide changed bytes, even before the first build snapshot.
  fs.appendFileSync(path.join(f.checkout, 'src/lib.rs'), '// changed\n');
  assert.throws(() => rxing.sourceFiles(f.root, f.metadata), /pinned Git blob/);
  assert.throws(() => rxing.directory(f.root, f.pkg), /pinned Git blob/);
});

test('the resolved graph cannot silently enable client support, encoders or extra formats', t => {
  const f = fixture(t);
  for (const features of [[], ['decoders', 'qrcode'], ['decoders', 'encoding_rs', 'qrcode', 'client_support'],
    ['decoders', 'encoding_rs', 'qrcode', 'encoders'], ['decoders', 'encoding_rs', 'qrcode', 'oned']]) {
    f.metadata.resolve.nodes[0].features = features;
    assert.throws(() => rxing.sourceFiles(f.root, f.metadata), /resolved features/);
  }
  f.metadata.resolve.nodes = [];
  assert.throws(() => rxing.sourceFiles(f.root, f.metadata), /resolved features/);
});

test('linked source directories cannot escape the approved checkout', t => {
  const f = fixture(t);
  const outside = path.join(f.root, 'outside');
  fs.mkdirSync(outside);
  fs.writeFileSync(path.join(outside, 'injected.rs'), 'not a checkout input');
  fs.symlinkSync(outside, path.join(f.checkout, 'src/linked'), 'junction');
  assert.throws(() => rxing.sourceFiles(f.root, f.metadata), /Linked rxing build input/);
});

test('actual product inventory contains rxing sources and original Apache license', () => {
  const metadataPath = process.env.OTTER_NATIVE_LICENSE_METADATA;
  assert.ok(metadataPath, 'Set OTTER_NATIVE_LICENSE_METADATA to locked Cargo metadata');
  const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
  const source = snapshot(repo, metadata);
  const files = rxing.sourceFiles(repo, metadata);
  assert.ok(files.some(row => row.path === 'third-party-git/rxing/src/lib.rs'));
  assert.ok(files.every(row => source.files.some(file => file.path === row.path && file.sha256 === row.sha256)));
  const unknown = { ...metadata, packages: [...metadata.packages,
    { name: 'unknown-git', version: '1.0.0', source: rxing.SOURCE }] };
  assert.throws(() => snapshot(repo, unknown), /Uninventoried Git/);
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  const parent = fs.mkdtempSync(path.join(base, 'rxing-license-'));
  const inventory = collect(repo, metadata, path.join(parent, 'materials'), path.join(cargoHome, 'registry/src'));
  const pkg = inventory.packages.find(entry => entry.name === rxing.NAME);
  assert.equal(pkg.expression, 'Apache-2.0');
  const license = pkg.files.find(file => file.path === `rxing-${rxing.VERSION}/LICENSE`);
  assert.ok(license);
  const original = fs.readFileSync(path.join(rxing.directory(repo, metadata.packages.find(p => p.name === rxing.NAME)), 'LICENSE'));
  assert.equal(license.sha256, hash(original));
  assert.deepEqual(fs.readFileSync(path.join(parent, 'materials', license.path)), original);
});
