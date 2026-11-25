// 验证原子台账写入工具的自动摘要绑定、去重排序、路径刷新及未知参数快速拒绝（不阻塞 stdin）契约。
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { normalizeEntries, parseCliArgs, refreshLedgerPaths, upsertReviewLedger } from '../review-ledger-upsert.mjs';
import { hashSourceBytes, scanRepositoryHashes } from '../review-coverage.mjs';
import { scanRepository } from '../effective-code-lines.mjs';

test('normalizeEntries binds exact inventory sha256 and rejects invalid outcome or missing finding', () => {
  const inv = new Map([['native/otter-core/src/pixels.rs', { sourceSha256: 'a'.repeat(64) }]]);
  const res = normalizeEntries(
    [{ path: 'native\\otter-core\\src\\pixels.rs', outcome: 'retained', finding: 'Arc lease verified.' }],
    inv,
  );
  assert.equal(res.get('native/otter-core/src/pixels.rs').sourceSha256, 'a'.repeat(64));

  assert.throws(() => normalizeEntries([{ path: 'absent.rs', outcome: 'retained', finding: 'ok' }], inv), /absent/);
  assert.throws(() => normalizeEntries([{ path: 'native/otter-core/src/pixels.rs', outcome: 'skip', finding: 'ok' }], inv), /outcome/);
  assert.throws(() => normalizeEntries([{ path: 'native/otter-core/src/pixels.rs', outcome: 'retained', finding: '  ' }], inv), /finding/);
  assert.throws(() => normalizeEntries([
    { path: 'native/otter-core/src/pixels.rs', outcome: 'changed', finding: 'first' },
    { path: 'native\\otter-core\\src\\pixels.rs', outcome: 'retained', finding: 'second' },
  ], inv), /Duplicate review update/);
});

test('parseCliArgs supports --entries, --refresh-paths and rejects unknown flags without blocking stdin', () => {
  assert.throws(() => parseCliArgs([]), /Usage:/);
  assert.throws(() => parseCliArgs(['--unknown-flag']), /Usage:/);
  assert.deepEqual(parseCliArgs(['--refresh-paths', 'a.rs', 'b.rs']), {
    mode: 'refresh',
    paths: ['a.rs', 'b.rs'],
  });
  assert.deepEqual(parseCliArgs(['--entry', 'retained|src/a.rs|verified clean']), {
    mode: 'upsert',
    entries: [{ outcome: 'retained', path: 'src/a.rs', finding: 'verified clean' }],
  });
});

test('hashSourceBytes and scanRepositoryHashes match official scanRepository LF sha256 exactly', () => {
  const crlf = Buffer.from('pub fn x() -> i32 {\r\n  1\r\n}\r\n', 'utf8');
  const lf = Buffer.from('pub fn x() -> i32 {\n  1\n}\n', 'utf8');
  assert.equal(hashSourceBytes('src/x.rs', crlf).sourceSha256, hashSourceBytes('src/x.rs', lf).sourceSha256);

  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'otter-hash-parity-'));
  try {
    fs.mkdirSync(path.join(tmp, 'native/src/desktop'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'native/otter-core/src'), { recursive: true });
    const policy = JSON.parse(fs.readFileSync(new URL('../effective-code-lines-policy.json', import.meta.url), 'utf8'));
    fs.writeFileSync(path.join(tmp, 'native/src/m.rs'), 'pub struct M;\r\n');
    fs.writeFileSync(path.join(tmp, 'native/src/desktop/d.rs'), 'pub struct D;\n');
    fs.writeFileSync(path.join(tmp, 'native/otter-core/src/c.rs'), 'pub struct C;\r\n');

    const full = scanRepository(tmp, policy);
    const fast = scanRepositoryHashes(tmp, policy);
    assert.deepEqual(
      fast.rows.map(r => [r.path, r.sourceSha256]),
      full.rows.map(r => [r.path, r.sourceSha256]),
    );

    const modelOnly = scanRepositoryHashes(tmp, policy, { filterGroup: 'native-model' });
    assert.deepEqual(modelOnly.rows.map(r => r.path), ['native/src/m.rs']);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test('upsertReviewLedger and refreshLedgerPaths update entries atomically with sorted output', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'otter-ledger-upsert-'));
  try {
    fs.mkdirSync(path.join(tmp, 'scripts/tests/fixtures/effective-code-lines'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'docs/development'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'src'), { recursive: true });
    const realPolicy = fs.readFileSync(new URL('../effective-code-lines-policy.json', import.meta.url), 'utf8');
    fs.writeFileSync(path.join(tmp, 'scripts/effective-code-lines-policy.json'), realPolicy);
    fs.writeFileSync(path.join(tmp, 'src/b.rs'), 'pub fn b() -> i32 { 2 }\r\n');
    fs.writeFileSync(path.join(tmp, 'src/a.rs'), 'pub fn a() -> i32 { 1 }\n');
    fs.writeFileSync(path.join(tmp, 'docs/development/review-coverage.json'), JSON.stringify({ schemaVersion: 1, files: [] }));

    const first = upsertReviewLedger(tmp, [
      { path: 'src/b.rs', outcome: 'retained', finding: 'b ok' },
      { path: 'src/a.rs', outcome: 'changed', finding: 'a fixed' },
    ]);
    assert.equal(first.inserted, 2);
    assert.equal(first.reviewed, 2);
    assert.deepEqual(first.stale, []);

    const ledgerPath = path.join(tmp, 'docs/development/review-coverage.json');
    const saved = JSON.parse(fs.readFileSync(ledgerPath, 'utf8'));
    assert.deepEqual(saved.files.map(f => f.path), ['src/a.rs', 'src/b.rs']);

    fs.writeFileSync(path.join(tmp, 'src/a.rs'), 'pub fn a() -> i32 { 42 }\n');
    const refreshed = refreshLedgerPaths(tmp, ['src/a.rs']);
    assert.equal(refreshed.updated, 1);
    assert.deepEqual(refreshed.stale, []);

    const snapshot = fs.readFileSync(ledgerPath, 'utf8');
    fs.writeFileSync(path.join(tmp, 'src/c.rs'), 'pub struct C;\n');
    assert.throws(() => refreshLedgerPaths(tmp, ['src/a.rs', 'src/c.rs']), /absent from review ledger/);
    assert.equal(fs.readFileSync(ledgerPath, 'utf8'), snapshot);

    const update = [{ path: 'src/a.rs', outcome: 'changed', finding: 'a fixed' }];
    for (const [extra, error] of [
      [saved.files[0], /Duplicate review entry/],
      [{ ...saved.files[0], path: 'src/removed.rs' }, /absent from repository inventory/],
    ]) {
      const invalid = JSON.stringify({ ...saved, files: [...saved.files, extra] });
      fs.writeFileSync(ledgerPath, invalid);
      assert.throws(() => upsertReviewLedger(tmp, update), error);
      assert.throws(() => refreshLedgerPaths(tmp, ['src/a.rs']), error);
      assert.equal(fs.readFileSync(ledgerPath, 'utf8'), invalid);
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
