// Deterministic file mutation at the inspection boundary; never race a live build or shared cache.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { readBytes } = require('../native-candidate/input-bytes.cjs');

function fixture(t, content = 'original') {
  const base = process.env.OTTER_NATIVE_CANDIDATE_TEST_ROOT;
  if (!base || !path.isAbsolute(base)) throw new Error('Set absolute OTTER_NATIVE_CANDIDATE_TEST_ROOT');
  fs.mkdirSync(base, { recursive: true });
  const root = fs.mkdtempSync(path.join(base, 'candidate-input-'));
  const file = path.join(root, 'input');
  fs.writeFileSync(file, content);
  const inspect = fs.fstatSync;
  const close = fs.closeSync;
  const state = { descriptor: undefined, closed: 0 };
  t.mock.method(fs, 'fstatSync', fd => {
    state.descriptor = fd;
    return inspect(fd);
  });
  t.mock.method(fs, 'closeSync', fd => { state.closed++; return close(fd); });
  return { root, file, state, inspect };
}

test('path replacement after inspection cannot replace the opened candidate input', t => {
  const { root, file, state, inspect } = fixture(t);
  t.mock.method(fs, 'fstatSync', fd => {
    state.descriptor = fd;
    const stat = inspect(fd);
    fs.renameSync(file, path.join(root, 'original'));
    fs.writeFileSync(file, 'replacement'.repeat(1000));
    return stat;
  });
  assert.equal(readBytes(file, 8).toString(), 'original');
  assert.equal(state.closed, 1);
  assert.throws(() => inspect(state.descriptor), { code: 'EBADF' });
});

for (const change of ['grow', 'shrink']) {
  test(`${change} after inspection fails within the original read budget`, t => {
    const { file, state, inspect } = fixture(t);
    t.mock.method(fs, 'fstatSync', fd => {
      state.descriptor = fd;
      const stat = inspect(fd);
      fs.writeFileSync(file, change === 'grow' ? 'x'.repeat(65536) : '');
      return stat;
    });
    const read = fs.readSync;
    let requested = 0;
    t.mock.method(fs, 'readSync', (fd, buffer, offset, length, position) => {
      requested += length;
      return read(fd, buffer, offset, length, position);
    });
    assert.throws(() => readBytes(file, 8), /changed while reading/);
    assert.ok(requested <= 9);
    assert.equal(state.closed, 1);
    assert.throws(() => inspect(state.descriptor), { code: 'EBADF' });
  });
}

test('partial reads preserve UTF-8 and exact boundary sizes', t => {
  const { file } = fixture(t, '候选证据');
  const read = fs.readSync;
  t.mock.method(fs, 'readSync', (fd, bytes, offset, length, position) =>
    read(fd, bytes, offset, Math.min(2, length), position));
  assert.equal(readBytes(file, 12, 1).toString('utf8'), '候选证据');
  assert.throws(() => readBytes(file, 11), /Invalid candidate input size/);
});

test('empty, missing, oversized and non-regular inputs respect each caller budget', t => {
  const { root, file } = fixture(t, '');
  assert.equal(readBytes(file, 0).length, 0);
  assert.throws(() => readBytes(file, 4096, 1), /Invalid candidate input size/);
  fs.truncateSync(file, 4097);
  assert.throws(() => readBytes(file, 4096), /Invalid candidate input size/);
  assert.throws(() => readBytes(root, 4096));
  assert.throws(() => readBytes(path.join(root, 'missing'), 4096), { code: 'ENOENT' });
});

for (const failure of ['fstatSync', 'readSync']) {
  test(`descriptor is closed after ${failure} throws`, t => {
    const { file, state } = fixture(t);
    t.mock.method(fs, failure, () => { throw new Error('synthetic failure'); });
    assert.throws(() => readBytes(file, 8), /synthetic failure/);
    assert.equal(state.closed, 1);
  });
}
