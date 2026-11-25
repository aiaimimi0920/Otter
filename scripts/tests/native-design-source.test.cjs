// 第一方主题的路径白名单、输入身份与许可证合同；夹具只写入显式临时根。
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const design = require('../native-candidate/design-source.cjs');
const { snapshot } = require('../native-candidate/source.cjs');
const { collect } = require('../native-candidate/licenses.cjs');
const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
fs.mkdirSync(base, { recursive: true });
const metadataPath = process.env.OTTER_NATIVE_LICENSE_METADATA;
assert.ok(metadataPath, 'Set OTTER_NATIVE_LICENSE_METADATA to locked Cargo metadata');
const metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
const repo = path.resolve(__dirname, '../..');
const pkg = metadata.packages.find(entry => entry.name === design.NAME);
assert.ok(pkg, 'The resolved graph must contain the native theme');

test('only the locked first-party Git commit and Cargo checkout can provide the theme', () => {
  assert.match(pkg.source, /^git\+https:\/\/github\.com\/aiaimimi0920\/NeuroDesign\?rev=([a-f0-9]{40})#\1$/);
  assert.ok(design.directory(repo, pkg).endsWith(path.join('windows-native', design.NAME)));
  for (const changes of [{ name: 'unknown-local' }, { version: '0.2.0' }, { source: null },
    { source: pkg.source.replace('/NeuroDesign?', '/Other?') },
    { source: pkg.source.replace(/[a-f0-9]{40}/g, 'a'.repeat(40)) },
    { manifest_path: path.join(repo, 'native/Cargo.toml') }]) {
    assert.throws(() => design.directory(repo, { ...pkg, ...changes }), /identity|mismatch|lockfile/);
  }
  assert.throws(() => design.sourceFiles(repo, { packages: [pkg, pkg] }), /Duplicate/);
  assert.throws(() => design.sourceFiles(repo, { packages: [] }), /Missing/);
});

test('actual product source and license inventory include the locked theme', () => {
  const source = snapshot(repo, metadata);
  const designFiles = design.sourceFiles(repo, metadata);
  assert.ok(designFiles.some(row => row.path === 'design-system/neuro-mini-ui/src/lib.rs'));
  assert.ok(designFiles.every(row => source.files.some(file => file.path === row.path && file.sha256 === row.sha256)));
  const paths = new Set(source.files.map(row => row.path));
  for (const name of ['native/assets/edit-strip/Settings.svg', 'native/assets/branding/icon.ico',
    'native/otter-runtime/src/tile_outputs.rs', 'native/src/tile_live_wire.rs', 'native/tests/fixtures/wall-media.v1.json']) {
    assert.ok(paths.has(name), `Missing build input: ${name}`);
  }
  assert.ok(!paths.has('scripts/export-native-edit-icons.cjs'));
  const unknown = { ...metadata, packages: [...metadata.packages, { name: 'unknown-local', version: '1.0.0', source: null }] };
  assert.throws(() => snapshot(repo, unknown), /Uninventoried local/);
  const registry = path.join(process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo'), 'registry/src');
  const parent = fs.mkdtempSync(path.join(base, 'theme-license-'));
  const inventory = collect(repo, metadata, path.join(parent, 'materials'), registry);
  const theme = inventory.packages.find(pkg => pkg.name === design.NAME);
  assert.equal(theme.expression, 'MIT');
  assert.ok(theme.files.some(file => file.path === 'neuro-mini-ui-0.1.0/LICENSE'));
});
