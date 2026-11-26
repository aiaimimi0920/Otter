// Reviewed Fluent dependency archives omit their upstream license texts; never substitute generic templates.
const path = require('node:path');
const crypto = require('node:crypto');
const { checkedPath } = require('../release/third-party-sources.cjs');
const { readBytes } = require('./input-bytes.cjs');
const REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index';
const PINS = {
  'fluent-langneg': {
    version: '0.13.1', license: 'Apache-2.0 OR MIT', repo: 'projectfluent/fluent-langneg-rs',
    revision: '7fdc590466ea63ca2367ac10fb1b881fe561f1a8', crate: '',
    files: {
      'LICENSE-MIT': '016fcc02048817d628b04b140c54db290fd9d29a5e8a55b77871194e810716ac',
      'LICENSE-APACHE': 'b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1',
    },
  },
  intl_pluralrules: {
    version: '7.0.2', license: 'Apache-2.0/MIT', repo: 'zbraniecki/pluralrules',
    revision: 'd0715d5b7cca72e228d6e6c4de154bd30dfdd06a', crate: 'intl_pluralrules',
    files: {
      'LICENSE-MIT': '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3',
      'LICENSE-APACHE': '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a',
    },
  },
};

function read(root, relative, limit) {
  const file = checkedPath(root, relative);
  const bytes = readBytes(file, limit, 1);
  new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  return bytes;
}

function fluentLicenses(pkg, registryRoot, materialsRoot = path.join(__dirname, 'license-texts')) {
  if (!Object.hasOwn(PINS, pkg.name)) return [];
  const pin = PINS[pkg.name];
  if (pkg.version !== pin.version || pkg.license !== pin.license || pkg.source !== REGISTRY) {
    throw new Error('Unreviewed Fluent license identity');
  }
  const relative = path.relative(registryRoot, path.dirname(pkg.manifest_path)).split(path.sep).join('/');
  const directory = checkedPath(registryRoot, relative);
  const vcs = JSON.parse(read(directory, '.cargo_vcs_info.json', 4096).toString('utf8'));
  if (vcs.git?.sha1 !== pin.revision || vcs.git?.dirty === true || vcs.path_in_vcs !== pin.crate) {
    throw new Error('Fluent crate VCS identity mismatch');
  }
  return Object.entries(pin.files).map(([name, expected]) => {
    const bytes = read(materialsRoot, `${pkg.name}-${pkg.version}/${name}`, 16 * 1024);
    const sha256 = crypto.createHash('sha256').update(bytes).digest('hex');
    if (sha256 !== expected) throw new Error('Fluent license original mismatch');
    const source = [pin.repo, pin.revision, pin.crate, name].filter(Boolean).join('/');
    return { name, bytes, sha256, upstream: `https://raw.githubusercontent.com/${source}` };
  });
}
module.exports = { fluentLicenses };
