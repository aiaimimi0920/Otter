// Admit only the pinned first-party Git crate from Cargo's checkout cache.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { checkedPath } = require('../release/third-party-sources.cjs');
const NAME = 'neuro-mini-ui';
const VERSION = '0.1.0';
const CRATE = 'packages/windows-native/neuro-mini-ui';
const SOURCE = /^git\+https:\/\/github\.com\/aiaimimi0920\/NeuroDesign\?rev=([a-f0-9]{40})#\1$/;

function directory(repo, pkg) {
  const pin = SOURCE.exec(pkg.source);
  if (pkg.name !== NAME || pkg.version !== VERSION || !pin) {
    throw new Error('Invalid Neuro-mini native dependency identity');
  }
  const lock = fs.readFileSync(checkedPath(repo, 'native/Cargo.lock'), 'utf8').replace(/\r\n/g, '\n');
  if (!lock.includes(`[[package]]\nname = "${NAME}"\nversion = "${VERSION}"\nsource = "${pkg.source}"\n`)) {
    throw new Error('Neuro-mini Git source does not match the lockfile');
  }
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  const checkouts = path.resolve(cargoHome, 'git/checkouts');
  const relative = path.relative(checkouts, pkg.manifest_path).split(path.sep).join('/');
  const parts = relative.split('/');
  if (parts.length !== 6 || !/^neurodesign-[a-f0-9]+$/.test(parts[0]) ||
      !/^[a-f0-9]{7,40}$/.test(parts[1]) || !pin[1].startsWith(parts[1]) ||
      parts.slice(2).join('/') !== `${CRATE}/Cargo.toml`) {
    throw new Error('Neuro-mini native dependency resolution mismatch');
  }
  const expected = checkedPath(checkouts, relative);
  const checkout = path.join(checkouts, ...parts.slice(0, 2));
  const git = (...args) => execFileSync('git', ['--no-optional-locks', '-C', checkout, ...args], { encoding: 'utf8' }).trim();
  if (git('rev-parse', 'HEAD') !== pin[1] || git('status', '--porcelain', '--untracked-files=all',
    '--ignored=matching', '--', `${CRATE}/Cargo.toml`, `${CRATE}/LICENSE`, `${CRATE}/src`)) {
    throw new Error('Neuro-mini checkout is dirty or does not match the pinned commit');
  }
  return path.dirname(expected);
}

function sourceFiles(repo, metadata) {
  const selected = metadata.packages.filter(pkg => pkg.name === NAME);
  if (selected.length > 1) throw new Error('Duplicate Neuro-mini native dependency');
  if (!selected.length) throw new Error('Missing Neuro-mini native dependency');
  const root = directory(repo, selected[0]);
  const files = ['Cargo.toml', 'LICENSE'];
  function visit(relative) {
    for (const entry of fs.readdirSync(checkedPath(root, relative), { withFileTypes: true })) {
      const name = `${relative}/${entry.name}`;
      if (entry.isSymbolicLink()) throw new Error('Linked Neuro-mini build input');
      if (entry.isDirectory()) visit(name);
      else if (entry.isFile()) files.push(name);
      else throw new Error('Unsupported Neuro-mini build input');
    }
  }
  visit('src');
  return files.sort().map(name => {
    const bytes = fs.readFileSync(checkedPath(root, name));
    return { path: `design-system/${NAME}/${name}`, bytes: bytes.length,
      sha256: crypto.createHash('sha256').update(bytes).digest('hex') };
  });
}
module.exports = { NAME, VERSION, directory, sourceFiles };
