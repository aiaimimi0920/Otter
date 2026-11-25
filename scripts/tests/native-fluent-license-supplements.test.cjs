// Exact Fluent license originals and VCS identities; tamper only with explicit temporary copies.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { fluentLicenses } = require('../native-candidate/fluent_licenses.cjs');
const { hash } = require('../native-candidate/source.cjs');
const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
const metadata = JSON.parse(fs.readFileSync(process.env.OTTER_NATIVE_LICENSE_METADATA, 'utf8'));
const originals = path.resolve(__dirname, '../native-candidate/license-texts');
const names = ['fluent-langneg', 'intl_pluralrules'];
fs.mkdirSync(base, { recursive: true });

function fixture() {
  const root = fs.mkdtempSync(path.join(base, 'fluent-licenses-'));
  const registry = path.join(root, 'registry');
  const materials = path.join(root, 'materials');
  const packages = names.map(name => {
    const original = metadata.packages.find(pkg => pkg.name === name);
    assert.ok(original);
    const key = `${name}-${original.version}`;
    const directory = path.join(registry, key);
    fs.mkdirSync(directory, { recursive: true });
    fs.copyFileSync(path.join(path.dirname(original.manifest_path), '.cargo_vcs_info.json'),
      path.join(directory, '.cargo_vcs_info.json'));
    fs.mkdirSync(path.join(materials, key), { recursive: true });
    for (const file of ['LICENSE-MIT', 'LICENSE-APACHE']) {
      fs.copyFileSync(path.join(originals, key, file), path.join(materials, key, file));
    }
    return { ...original, manifest_path: path.join(directory, 'Cargo.toml') };
  });
  return { registry, materials, packages };
}

test('both omitted upstream license alternatives retain exact bytes and source URLs', () => {
  const f = fixture();
  for (const pkg of f.packages) {
    const files = fluentLicenses(pkg, f.registry, f.materials);
    assert.deepEqual(files.map(file => file.name), ['LICENSE-MIT', 'LICENSE-APACHE']);
    for (const file of files) {
      assert.equal(file.sha256, hash(file.bytes));
      assert.match(file.upstream, /^https:\/\/raw\.githubusercontent\.com\/[^/]+\/[^/]+\/[a-f0-9]{40}\//);
      assert.deepEqual(file.bytes, fs.readFileSync(path.join(originals, `${pkg.name}-${pkg.version}`, file.name)));
    }
  }
  assert.deepEqual(fluentLicenses({ name: 'unrelated' }, f.registry, f.materials), []);
});

test('new versions, alternate licenses and changed registry sources have no implicit fallback', () => {
  const f = fixture();
  for (const pkg of f.packages) {
    for (const changes of [{ version: '999.0.0' }, { license: 'MIT' }, { source: 'git+untrusted' }]) {
      assert.throws(() => fluentLicenses({ ...pkg, ...changes }, f.registry, f.materials), /Unreviewed/);
    }
  }
});

test('changed VCS revisions, crate paths and dirty archives fail closed', () => {
  for (const name of names) {
    for (const change of [vcs => vcs.git.sha1 = 'a'.repeat(40),
      vcs => vcs.path_in_vcs = 'unrelated', vcs => vcs.git.dirty = true]) {
      const f = fixture();
      const pkg = f.packages.find(pkg => pkg.name === name);
      const file = path.join(path.dirname(pkg.manifest_path), '.cargo_vcs_info.json');
      const vcs = JSON.parse(fs.readFileSync(file, 'utf8'));
      change(vcs);
      fs.writeFileSync(file, JSON.stringify(vcs));
      assert.throws(() => fluentLicenses(pkg, f.registry, f.materials), /VCS identity/);
    }
  }
});

test('tampered original MIT and Apache texts fail their reviewed digests', () => {
  for (const name of names) {
    for (const file of ['LICENSE-MIT', 'LICENSE-APACHE']) {
      const f = fixture();
      const pkg = f.packages.find(pkg => pkg.name === name);
      fs.appendFileSync(path.join(f.materials, `${pkg.name}-${pkg.version}`, file), 'modified');
      assert.throws(() => fluentLicenses(pkg, f.registry, f.materials), /original mismatch/);
    }
  }
});

test('registry escapes and linked VCS roots are rejected', () => {
  const f = fixture();
  const pkg = f.packages[0];
  assert.throws(() => fluentLicenses({ ...pkg, manifest_path: path.join(f.registry, '../outside/Cargo.toml') },
    f.registry, f.materials), /Invalid third-party path/);
  fs.symlinkSync(path.dirname(pkg.manifest_path), path.join(f.registry, 'linked'), 'junction');
  assert.throws(() => fluentLicenses({ ...pkg, manifest_path: path.join(f.registry, 'linked/Cargo.toml') },
    f.registry, f.materials), /Linked source/);
});
