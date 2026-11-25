// Per-document rAF queue; runs only when the host explicitly delivers a frame, with new requests from callbacks deferred to the next frame.
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
        // Canceling a same-frame callback that has not run takes effect; new requests do not enter this handle snapshot.
        op.apply(callback, globalThis, args);
        // Rust drains microtasks before the next callback, so Promise-based cancellation also precedes subsequent callbacks.
        return true;
      }
      handles = args = null; return false;
    },
    dispose() { disposed = true; op.mapClear(callbacks); clock = undefined; handles = args = null; },
  };
})
