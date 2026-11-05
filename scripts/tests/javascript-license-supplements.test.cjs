// 精确共享许可证与 vendored 引擎声明；只修改显式 linshi 测试根下的副本。
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { javascriptLicenses } = require('../native-candidate/javascript_licenses.cjs');
const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
const metadata = JSON.parse(fs.readFileSync(process.env.OTTER_NATIVE_LICENSE_METADATA, 'utf8'));
fs.mkdirSync(base, { recursive: true });
function fixture() {
  const root = fs.mkdtempSync(path.join(base, 'javascript-licenses-'));
  const packages = ['rquickjs', 'rquickjs-core', 'rquickjs-sys'].map(name => {
    const original = metadata.packages.find(p => p.name === name && p.version === '0.14.0');
    assert.ok(original);
    const directory = path.join(root, `${name}-0.14.0`);
    fs.mkdirSync(directory);
    if (name === 'rquickjs' || name === 'rquickjs-sys') {
      const relative = name === 'rquickjs' ? 'LICENSE' : 'quickjs/LICENSE';
      fs.mkdirSync(path.dirname(path.join(directory, relative)), { recursive: true });
      fs.copyFileSync(path.join(path.dirname(original.manifest_path), relative), path.join(directory, relative));
    }
    return { ...original, manifest_path: path.join(directory, 'Cargo.toml') };
  });
  return { root, metadata: { packages }, core: packages[1], sys: packages[2] };
}
test('reviewed core/sys share original MIT and sys retains original QuickJS notice', () => {
  const f = fixture();
  const core = javascriptLicenses(f.core, f.metadata, f.root);
  const sys = javascriptLicenses(f.sys, f.metadata, f.root);
  assert.deepEqual(core.map(p => p.name), ['LICENSE']);
  assert.deepEqual(sys.map(p => p.name), ['LICENSE', 'quickjs/LICENSE']);
  assert.deepEqual(core[0].bytes, sys[0].bytes);
  assert.match(sys[1].bytes.toString('utf8'), /Saúl Ibarra Corretgé/);
  assert.ok(sys.every(p => p.upstream.startsWith('https://raw.githubusercontent.com/')));
  assert.deepEqual(javascriptLicenses({ name: 'unrelated' }, {}, f.root), []);
});
test('changed original MIT or nested QuickJS bytes fail closed', () => {
  for (const name of ['rquickjs-0.14.0/LICENSE', 'rquickjs-sys-0.14.0/quickjs/LICENSE']) {
    const f = fixture();
    fs.appendFileSync(path.join(f.root, name), 'modified');
    assert.throws(() => javascriptLicenses(f.sys, f.metadata, f.root), /original mismatch/);
  }
});
test('different versions, license expressions and registry sources are not aliases', () => {
  const f = fixture();
  for (const change of [{ version: '0.14.1' }, { license: 'MIT OR Apache-2.0' }, { source: 'git+untrusted' }]) {
    assert.throws(() => javascriptLicenses({ ...f.core, ...change }, f.metadata, f.root), /Unreviewed/);
  }
});
test('missing/duplicate peer and escaped registry paths are rejected', () => {
  const f = fixture();
  assert.throws(() => javascriptLicenses(f.core, { packages: [] }, f.root), /peer/);
  assert.throws(() => javascriptLicenses(f.core, { packages: [f.metadata.packages[0], f.metadata.packages[0]] }, f.root), /peer/);
  f.metadata.packages[0].manifest_path = path.join(f.root, '../outside/Cargo.toml');
  assert.throws(() => javascriptLicenses(f.core, f.metadata, f.root));
});
