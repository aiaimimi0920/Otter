// 单文档 rAF 队列；只有宿主明确交付一帧时执行，回调中新请求留到下一帧。
((dom, fail) => {
  "use strict";
  const op = dom.ops, callbacks = new op.MapType();
  let clock, next = 0, disposed = false;
  let handles = null, args = null, index = 0;
  const now = () => { if (!clock || disposed) fail(); return clock(); };
  const request = callback => {
    if (disposed || typeof callback !== "function" || op.mapSize(callbacks) >= 64 || next === 0xffffffff) fail();
    const handle = ++next; op.mapSet(callbacks, handle, callback); return handle;
  };
  const cancel = handle => { op.mapDelete(callbacks, op.number(handle) >>> 0); };
  Object.defineProperties(globalThis, {
    requestAnimationFrame: { value: request },
    cancelAnimationFrame: { value: cancel },
    performance: { value: op.freeze({ now }) },
  });
  return {
    now,
    bind(value) { if (clock || typeof value !== "function") fail(); clock = value; },
    pending: () => !disposed && op.mapSize(callbacks) > 0,
    begin() {
      if (disposed || handles) fail();
      handles = op.array(); args = op.array(); index = 0; op.push(args, now());
      op.mapEach(callbacks, (_, handle) => op.push(handles, handle));
    },
    step() {
      if (disposed || !handles) fail();
      while (index < handles.length) {
        const handle = handles[index++], callback = op.mapGet(callbacks, handle);
        if (!callback) continue;
        op.mapDelete(callbacks, handle);
        // 取消尚未执行的同帧回调生效；新请求不进入本次 handle 快照。
        op.apply(callback, globalThis, args);
        // Rust 在下一 callback 前排尽 microtask，使 Promise 取消也先于后续 callback。
        return true;
      }
      handles = args = null; return false;
    },
    dispose() { disposed = true; op.mapClear(callbacks); clock = undefined; handles = args = null; },
  };
})
