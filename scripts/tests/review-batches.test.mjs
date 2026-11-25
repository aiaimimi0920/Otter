// 验证审查分包与热点预筛脚本的分层归类、热点提取及批次切分契约。
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { buildReviewBatches, classifyGroup, moduleClusterKey, detectHotspots } from '../review-batches.mjs';

test('classifyGroup maps workspace subcrates and host layers accurately', () => {
  assert.equal(classifyGroup('native/otter-core/src/pixels.rs'), 'otter-core');
  assert.equal(classifyGroup('native/otter-media/src/capture_wgc.rs'), 'otter-media');
  assert.equal(classifyGroup('native/otter-runtime/src/editor.rs'), 'otter-runtime');
  assert.equal(classifyGroup('native/hover-hook/src/lib.rs'), 'hover-hook');
  assert.equal(classifyGroup('native/src/desktop/application.rs'), 'desktop');
  assert.equal(classifyGroup('native/src/session_write.rs'), 'native-model');
  assert.equal(classifyGroup('native/tests/integration.rs'), 'native-integration');
  assert.equal(classifyGroup('scripts/review-batches.mjs'), 'scripts');
});

test('moduleClusterKey pairs implementation and adjacent test files into the same cluster', () => {
  assert.equal(
    moduleClusterKey('native/otter-media/src/capture_readback.rs'),
    moduleClusterKey('native/otter-media/src/capture_readback_tests.rs'),
  );
  assert.equal(
    moduleClusterKey('native/src/desktop/application_live_smoke.rs'),
    'desktop:application_live',
  );
});

test('detectHotspots flags repeated JSON decode, clone bursts, and Win32 GDI/DPI calls', () => {
  const cleanCode = 'pub struct Rect { pub x: i32, pub y: i32 }\n';
  assert.deepEqual(detectHotspots(cleanCode), []);

  const hotCode = `
    let a = serde_json::from_str(&raw)?;
    let b = serde_json::to_vec(&a)?;
    let _ = x.clone(); let _ = y.clone(); let _ = z.to_vec(); let _ = w.to_owned();
    unsafe { GetWindowRect(hwnd, &mut rect); GetDpiForWindow(hwnd); unsafe {} unsafe {} }
  `;
  const tags = detectHotspots(hotCode);
  assert.ok(tags.some(t => t.startsWith('json_decode(')));
  assert.ok(tags.some(t => t.startsWith('alloc_clone(')));
  assert.ok(tags.some(t => t.startsWith('win32_query_gdi(')));
  assert.ok(tags.some(t => t.startsWith('unsafe_ffi(')));
});

test('buildReviewBatches skips reviewed entries and scopes group filtering accurately', () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'otter-batches-'));
  try {
    fs.mkdirSync(path.join(tmp, 'scripts'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'docs/development'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'native/otter-core/src'), { recursive: true });
    fs.mkdirSync(path.join(tmp, 'native/src/desktop'), { recursive: true });
    const policy = fs.readFileSync(new URL('../effective-code-lines-policy.json', import.meta.url), 'utf8');
    fs.writeFileSync(path.join(tmp, 'scripts/effective-code-lines-policy.json'), policy);
    fs.writeFileSync(path.join(tmp, 'native/otter-core/src/a.rs'), '// comment\npub fn a() {}\n');
    fs.writeFileSync(path.join(tmp, 'native/src/desktop/w.rs'), 'pub fn w() {}\n');
    fs.writeFileSync(path.join(tmp, 'docs/development/review-coverage.json'), JSON.stringify({ schemaVersion: 1, files: [] }));

    const coreOnly = buildReviewBatches(tmp, { group: 'otter-core' });
    assert.equal(coreOnly.totalPendingInScope, 1);
    assert.equal(coreOnly.nextBatches[0].cleanFiles[0], 'native/otter-core/src/a.rs');
    assert.equal(coreOnly.nextBatches[0].totalEffectiveLines, 1);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
