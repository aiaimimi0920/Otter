// 同文档 ResizeObserver：原生盒测量、分轮深度交付；无私有 timer、布局副本或空闲轮询。
((dom, fail, geometry) => {
  "use strict";
  const op = dom.ops, observers = new op.MapType();
  const define = Object.defineProperty, iterator = Array.prototype[Symbol.iterator], iteratorKey = Symbol.iterator;
  const sequence = () => define(op.array(), iteratorKey, { value: iterator });
  let targets = 0, disposed = false, batches = null, index = 0, depth = 0, nextDepth = Infinity;
  const state = observer => op.mapGet(observers, observer) ?? fail();
  const element = target => {
    const tag = dom.get(target).tag;
    if (tag === "#text" || tag === "#fragment") fail();
  };
  const depthOf = target => {
    let value = 0;
    for (let node = target; node; node = dom.get(node).parent) value++;
    return value;
  };
  const box = (width, height) => {
    const size = op.record(), list = sequence(); size.inlineSize = width; size.blockSize = height;
    op.push(list, op.freeze(size)); return op.freeze(list);
  };
  const entry = target => {
    const value = geometry.read(target), rect = op.record(), result = op.record();
    rect.x = rect.left = value[8]; rect.y = rect.top = value[9];
    rect.width = value[6]; rect.height = value[7];
    rect.right = rect.x + rect.width; rect.bottom = rect.y + rect.height;
    result.target = target; result.contentRect = op.freeze(rect);
    result.contentBoxSize = box(value[6], value[7]); result.borderBoxSize = box(value[2], value[3]);
    result.devicePixelContentBoxSize = box(value[10], value[11]);
    return op.freeze(result);
  };
  const sizeOf = (value, mode) => mode === "border-box" ? value.borderBoxSize[0] :
    mode === "device-pixel-content-box" ? value.devicePixelContentBoxSize[0] : value.contentBoxSize[0];
  class ResizeObserver {
    constructor(callback) {
      // 有界文档 lifetime registry；disconnect 释放所有目标，但不重复利用 observer 身份。
      if (disposed || typeof callback !== "function" || op.mapSize(observers) >= 32) fail();
      op.mapSet(observers, this, { callback, observed: new op.MapType() });
    }
    observe(target, options = undefined) {
      const owner = state(this); element(target);
      if (disposed || (options != null && typeof options !== "object")) fail();
      const requested = options?.box, mode = requested === undefined ? "content-box" : requested;
      if (mode !== "content-box" && mode !== "border-box" && mode !== "device-pixel-content-box") fail();
      const exists = op.mapHas(owner.observed, target);
      if (!exists && targets >= 64) fail();
      if (exists) op.mapDelete(owner.observed, target); else targets++;
      op.mapSet(owner.observed, target, { mode, width: -1, height: -1 });
    }
    unobserve(target) {
      const owner = state(this); element(target);
      if (op.mapDelete(owner.observed, target)) targets--;
    }
    disconnect() {
      const owner = state(this); targets -= op.mapSize(owner.observed); op.mapClear(owner.observed);
      // 另一个 observer 的回调可以在同一轮中取消尚未交付的整个批次。
      if (batches) for (let i = index; i < batches.length; i++)
        if (batches[i].observer === this) batches[i].active.length = 0;
    }
  }
  Object.defineProperty(globalThis, "ResizeObserver", { value: ResizeObserver });
  const gather = () => {
    batches = op.array(); index = 0; nextDepth = Infinity; let skipped = false;
    // 先收集本轮 active target；后续 observer 的 entry 在其 callback 前测量，反映前序修改。
    const owners = op.mapEntries(observers);
    for (;;) {
      const next = op.mapNext(owners); if (next.done) break;
      const observer = next.value[0], owner = next.value[1];
      const active = op.array();
      const observed = op.mapEntries(owner.observed);
      for (;;) {
        const nextTarget = op.mapNext(observed); if (nextTarget.done) break;
        const target = nextTarget.value[0], item = nextTarget.value[1];
        const value = sizeOf(entry(target), item.mode);
        if (value.inlineSize === item.width && value.blockSize === item.height) continue;
        if (depthOf(target) <= depth) { skipped = true; continue; }
        op.push(active, { target, item });
      }
      if (active.length) op.push(batches, { observer, active });
    }
    // 没有浏览器 ErrorEvent/下一渲染轮恢复合同；循环按现有执行失败策略终态拒绝。
    if (!batches.length && skipped) fail();
  };
  const step = () => {
    if (disposed) return false;
    if (!batches) gather();
    for (;;) {
      while (index < batches.length) {
        const batch = batches[index++], owner = state(batch.observer), entries = sequence();
        for (let i = 0; i < batch.active.length; i++) {
          const { target, item } = batch.active[i], value = entry(target), size = sizeOf(value, item.mode);
          item.width = size.inlineSize; item.height = size.blockSize;
          const targetDepth = depthOf(target); if (targetDepth < nextDepth) nextDepth = targetDepth;
          op.push(entries, value);
        }
        if (!entries.length) continue;
        const args = op.array(); op.push(args, entries); op.push(args, batch.observer);
        op.apply(owner.callback, batch.observer, args);
        // Rust 在下一 observer callback 前排尽 microtask；禁止 JS 内自建无限通知循环。
        return true;
      }
      if (!batches.length) { batches = null; return false; }
      depth = nextDepth; gather();
      if (!batches.length) { batches = null; return false; }
    }
  };
  return {
    observing: () => !disposed && targets > 0,
    begin() { depth = 0; batches = null; },
    step,
    dispose() { disposed = true; targets = 0; batches = null; op.mapClear(observers); },
  };
})
