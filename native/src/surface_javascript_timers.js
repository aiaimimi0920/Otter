// 单文档有界 timer；仅宿主交付 task 时执行，复用 rAF 的私有单调时钟。
((dom, fail, clock) => {
  "use strict";
  const op = dom.ops, timers = new op.MapType(), ceil = Math.ceil;
  let next = 0, disposed = false, paused = null, offset = 0, nesting = 0;
  let batch = null, index = 0;
  const now = () => (paused === null ? clock() : paused) - offset;
  const delay = (value, level) => {
    // Web IDL long：先 ToNumber（BigInt/Symbol 抛错），再 signed 32-bit，负数归零。
    let result = +value | 0;
    if (result < 0) result = 0;
    if (level > 5 && result < 4) result = 4;
    return result;
  };
  const register = (callback, timeout, args, repeat) => {
    if (disposed || typeof callback !== "function" || args.length > 32) fail();
    const raw = +timeout | 0, wait = delay(raw, nesting);
    // 转换可能重入注册或 dispose；在转换后才最终准入，不超容量或复用 ID。
    if (disposed || op.mapSize(timers) >= 64 || next === 0x7fffffff) fail();
    const saved = op.array();
    for (let i = 0; i < args.length; i++) op.push(saved, args[i]);
    const id = ++next;
    op.mapSet(timers, id, { callback, args: saved, timeout: raw,
      due: now() + wait, level: nesting + 1, repeat });
    return id;
  };
  const clear = id => { op.mapDelete(timers, +id | 0); };
  Object.defineProperties(globalThis, {
    setTimeout: { value: (callback, timeout = 0, ...args) => register(callback, timeout, args, false) },
    setInterval: { value: (callback, timeout = 0, ...args) => register(callback, timeout, args, true) },
    clearTimeout: { value: clear }, clearInterval: { value: clear },
  });
  return {
    begin() {
      if (disposed || paused !== null || batch) fail();
      batch = op.array(); index = 0; const timestamp = now();
      op.mapEach(timers, (timer, id) => {
        if (timer.due > timestamp) return;
        // 最多64项；稳定按到期时间/登记顺序，不调用插件可替换的 sort/iterator。
        let at = batch.length;
        while (at > 0 && batch[at - 1].timer.due > timer.due) { batch[at] = batch[at - 1]; at--; }
        batch[at] = { timer, id };
      });
    },
    step() {
      if (disposed || !batch || paused !== null) fail();
      nesting = 0;
      while (index < batch.length) {
        const item = batch[index++], timer = item.timer;
        if (op.mapGet(timers, item.id) !== timer) continue;
        nesting = timer.level;
        op.apply(timer.callback, globalThis, timer.args);
        if (op.mapGet(timers, item.id) === timer) {
          if (!timer.repeat) op.mapDelete(timers, item.id);
          else {
            timer.due = now() + delay(timer.timeout, nesting);
            timer.level = nesting + 1;
          }
        }
        // Rust 完成 microtask checkpoint 后才进入下一项；新登记留到下一 task。
        return true;
      }
      batch = null; return false;
    },
    pending() {
      if (disposed || paused !== null || !op.mapSize(timers)) return null;
      let wait = 0x7fffffff; const timestamp = now();
      op.mapEach(timers, timer => { const remaining = ceil(timer.due - timestamp); if (remaining < wait) wait = remaining; });
      return wait < 0 ? 0 : wait;
    },
    suspend() { if (paused === null) paused = clock(); },
    resume() { if (paused !== null) { offset += clock() - paused; paused = null; } },
    dispose() { disposed = true; batch = null; nesting = 0; op.mapClear(timers); },
  };
})
