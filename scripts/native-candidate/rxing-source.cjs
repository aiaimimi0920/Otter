// Admit only the reviewed upstream fix; never treat arbitrary Git packages as registry sources.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const childProcess = require('node:child_process');
const { checkedPath } = require('../release/third-party-sources.cjs');
const { readBytes } = require('./input-bytes.cjs');
const NAME = 'rxing';
const VERSION = '0.9.4';
const REVISION = '912b1083ffd164b29e6b244e4845de9df06b107a';
const SOURCE = `git+https://github.com/rxing-core/rxing?rev=${REVISION}#${REVISION}`;
const INPUTS = ['Cargo.toml', 'LICENSE', 'README.md', 'src', 'build.rs'];
const git = (checkout, ...args) => childProcess.execFileSync('git',
  ['--no-optional-locks', '--no-replace-objects', '-C', checkout, ...args],
  { encoding: 'utf8', windowsHide: true, timeout: 30000, maxBuffer: 1024 * 1024 }).trim();

function checkedDirectory(repo, pkg) {
  if (pkg.name !== NAME || pkg.version !== VERSION || pkg.source !== SOURCE || pkg.license !== 'Apache-2.0') {
    throw new Error('Invalid rxing Git dependency identity');
  }
  const lock = fs.readFileSync(checkedPath(repo, 'native/Cargo.lock'), 'utf8').replace(/\r\n/g, '\n');
  if (!lock.includes(`[[package]]\nname = "${NAME}"\nversion = "${VERSION}"\nsource = "${SOURCE}"\n`)) {
    throw new Error('rxing Git source does not match the lockfile');
  }
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  const checkouts = path.resolve(cargoHome, 'git/checkouts');
  const relative = path.relative(checkouts, pkg.manifest_path).split(path.sep).join('/');
  const parts = relative.split('/');
  if (parts.length !== 3 || !/^rxing-[a-f0-9]+$/.test(parts[0]) ||
      !/^[a-f0-9]{7,40}$/.test(parts[1]) || !REVISION.startsWith(parts[1]) || parts[2] !== 'Cargo.toml') {
    throw new Error('rxing Git dependency resolution mismatch');
  }
  const expected = checkedPath(checkouts, relative);
  const checkout = path.dirname(expected);
  // Cargo's cache-completion marker is not source. All other tracked, new and ignored changes fail.
  if (git(checkout, 'rev-parse', 'HEAD') !== REVISION || git(checkout, 'status', '--porcelain', '--untracked-files=all',
    '--ignored=matching', '--', '.', ':(exclude).cargo-ok')) {
    throw new Error('rxing checkout is dirty or does not match the pinned commit');
  }
  return checkout;
}

function selectedPackage(metadata) {
  const selected = metadata.packages.filter(pkg => pkg.name === NAME);
  if (selected.length > 1) throw new Error('Duplicate rxing Git dependency');
  if (!selected.length) throw new Error('Missing rxing Git dependency');
  const nodes = metadata.resolve?.nodes?.filter(node => node.id === selected[0].id);
  if (nodes?.length !== 1 || JSON.stringify([...nodes[0].features].sort()) !==
      JSON.stringify(['decoders', 'encoding_rs', 'qrcode'])) throw new Error('Unexpected rxing resolved features');
  return selected[0];
}

function verifiedFiles(root) {
  const expected = new Map();
  for (const row of git(root, 'ls-tree', '-rz', REVISION, '--', ...INPUTS).split('\0').filter(Boolean)) {
    const match = /^100(?:644|755) blob ([a-f0-9]{40})\t(.+)$/.exec(row);
    if (!match || expected.has(match[2])) throw new Error('Invalid rxing upstream source tree');
    expected.set(match[2], match[1]);
  }
  const files = ['Cargo.toml', 'LICENSE', 'README.md'];
  if (fs.existsSync(path.join(root, 'build.rs'))) files.push('build.rs');
  function visit(relative, depth) {
    if (depth > 32) throw new Error('Oversized rxing source tree');
    for (const entry of fs.readdirSync(checkedPath(root, relative), { withFileTypes: true })) {
      const name = `${relative}/${entry.name}`;
      if (entry.isSymbolicLink()) throw new Error('Linked rxing build input');
      if (entry.isDirectory()) visit(name, depth + 1);
      else if (entry.isFile()) files.push(name);
      else throw new Error('Unsupported rxing build input');
      if (files.length > 4096) throw new Error('Oversized rxing source tree');
    }
  }
  visit('src', 0);
  if (JSON.stringify(files.sort()) !== JSON.stringify([...expected.keys()].sort())) {
    throw new Error('rxing source inventory does not match the pinned commit');
  }
  let total = 0;
  return files.map(name => {
    const file = checkedPath(root, name);
    const bytes = readBytes(file, Math.min(4 * 1024 * 1024, 64 * 1024 * 1024 - total));
    total += bytes.length;
    // Check raw Git blobs, not index flags or clean filters that can hide worktree tampering.
    const blob = crypto.createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
    if (blob !== expected.get(name)) throw new Error(`rxing input differs from the pinned Git blob: ${name}`);
    return { path: `third-party-git/${NAME}/${name}`, bytes: bytes.length,
      sha256: crypto.createHash('sha256').update(bytes).digest('hex') };
  });
}

function directory(repo, pkg) {
  const root = checkedDirectory(repo, pkg);
  verifiedFiles(root);
  return root;
}

function sourceFiles(repo, metadata) {
  return verifiedFiles(checkedDirectory(repo, selectedPackage(metadata)));
}
module.exports = { NAME, VERSION, REVISION, SOURCE, directory, sourceFiles, selectedPackage };
