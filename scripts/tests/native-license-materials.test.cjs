// 图合同与目录边界测试；临时材料保留在显式指定的测试目录中。
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { selectedPackages, licenseFiles, collect } = require('../native-candidate/licenses.cjs');
const { hash } = require('../native-candidate/source.cjs');
const root = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!root || !path.isAbsolute(root)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
fs.mkdirSync(root, { recursive: true });
function graph() {
  const packages = ['root', 'normal', 'build', 'dev', 'shared'].map(id => ({ id, name: id === 'root' ? 'otter-native' : id }));
  const edge = (pkg, ...kinds) => ({ pkg, dep_kinds: kinds.map(kind => ({ kind })) });
  return { packages, resolve: { root: 'root', nodes: [
    { id: 'root', deps: [edge('normal', null), edge('build', 'build'), edge('dev', 'dev'), edge('shared', 'dev', null)] },
    { id: 'normal', deps: [edge('root', null)] }, ...['build', 'dev', 'shared'].map(id => ({ id, deps: [] }))
  ] } };
}
test('normal/build closure includes shared normal edges, handles cycles and excludes dev-only nodes', () => {
  assert.deepEqual(selectedPackages(graph()).map(p => p.id), ['build', 'normal', 'root', 'shared']);
});
test('incomplete, duplicate, unknown-kind and empty-kind graphs are rejected', () => {
  for (const change of [m => m.resolve.nodes.pop(), m => m.packages.push(m.packages[0]),
    m => m.resolve.nodes[0].deps[0].dep_kinds = [],
    m => m.resolve.nodes[0].deps[0].dep_kinds[0].kind = 'unknown', m => m.resolve.root = 'absent']) {
    const m = graph();
    change(m);
    assert.throws(() => selectedPackages(m), /license graph|dependency kind/);
  }
});
test('license inventory includes nested license directories but not arbitrary source files', () => {
  const directory = fs.mkdtempSync(path.join(root, 'license-tree-'));
  fs.mkdirSync(path.join(directory, 'LICENSES'));
  fs.writeFileSync(path.join(directory, 'LICENSES/MIT.txt'), 'test license');
  fs.writeFileSync(path.join(directory, 'NOTICE'), 'test notice');
  fs.writeFileSync(path.join(directory, 'source.rs'), 'not license material');
  assert.deepEqual(licenseFiles(directory), ['LICENSES/MIT.txt', 'NOTICE']);
});
test('actual native metadata produces byte-bound texts and rejects host path escape', () => {
  const metadataPath = process.env.OTTER_NATIVE_LICENSE_METADATA;
  assert.ok(metadataPath, 'Set OTTER_NATIVE_LICENSE_METADATA to offline Cargo metadata');
  const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
  const repo = path.resolve(__dirname, '../..');
  const registry = path.join(process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo'), 'registry/src');
  const parent = fs.mkdtempSync(path.join(root, 'license-collect-'));
  const output = path.join(parent, 'materials');
  const inventory = collect(repo, metadata, output, registry);
  assert.equal(inventory.legalClearance, false);
  assert.equal(inventory.packages.length, selectedPackages(metadata).length);
  assert.ok(!inventory.packages.some(p => p.name === 'rcgen' || p.name === 'yasna'));
  for (const pkg of inventory.packages) {
    assert.ok(pkg.files.length > 0);
    for (const file of pkg.files) {
      const bytes = fs.readFileSync(path.join(output, file.path));
      assert.equal(bytes.length, file.bytes);
      assert.equal(hash(bytes), file.sha256);
    }
  }
  assert.throws(() => collect(repo, metadata, output, registry), /already exists/);
  const selected = new Set(selectedPackages(metadata).map(p => p.id));
  const pkg = metadata.packages.find(p => p.source?.startsWith('registry+') && selected.has(p.id));
  pkg.manifest_path = path.join(repo, 'Cargo.toml');
  const rejected = path.join(parent, 'rejected');
  assert.throws(() => collect(repo, metadata, rejected, registry), /Invalid third-party path/);
  assert.equal(fs.existsSync(rejected), false);
});
