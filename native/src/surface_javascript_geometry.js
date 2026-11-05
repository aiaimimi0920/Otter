// 同步 border/client box 只取同一原生度量；没有 viewport/独立 inline run 时明确失败而非猜测。
((dom, fail, project, json) => {
  "use strict";
  const op = dom.ops, round = Math.round;
  let native, ratio = null, used = false, disposed = false, cachedRevision, cachedSource;
  const zero = () => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
  const read = node => {
    if (disposed) fail();
    dom.get(node);
    let current = node, connected = false;
    while (current) {
      if (current === dom.root) { connected = true; break; }
      current = dom.get(current).parent;
    }
    if (!connected) return zero();
    // 叶中的 inline 被现有投影压平，没有独立 box；禁止以叶/父矩形冒充它的尺寸。
    for (current = dom.get(node).parent; current; current = dom.get(current).parent)
      if (!op.setHas(dom.containers, dom.get(current).tag)) fail();
    if (!native || ratio === null) fail();
    used = true;
    const revision = dom.revision();
    if (revision !== cachedRevision) {
      const source = json.encode(project(dom.root));
      if (revision !== dom.revision()) fail();
      cachedSource = source; cachedRevision = revision;
    }
    // 只省同 operation 的重复投影；每次读取仍经过原生 reads/layout/deadline 准入。
    const result = native(cachedSource, dom.get(node).key);
    if (!result) fail();
    return json.decode(result);
  };
  Object.defineProperties(dom.classes.Element.prototype, {
    getBoundingClientRect: { value() {
      const value = read(this), result = op.record();
      result.x = result.left = value[0]; result.y = result.top = value[1];
      result.width = value[2]; result.height = value[3];
      result.right = value[0] + value[2]; result.bottom = value[1] + value[3];
      return op.freeze(result);
    } },
    clientWidth: { get() { return round(read(this)[4]); } },
    clientHeight: { get() { return round(read(this)[5]); } },
    offsetWidth: { get() { return round(read(this)[2]); } },
    offsetHeight: { get() { return round(read(this)[3]); } },
  });
  Object.defineProperty(globalThis, "devicePixelRatio", { get() {
    if (disposed || ratio === null) fail(); used = true; return ratio;
  } });
  return {
    read,
    bind(value) { if (native || typeof value !== "function") fail(); native = value; },
    begin(value) { cachedRevision = cachedSource = undefined; ratio = json.decode(value); used = false; },
    used: () => used,
    dispose() { disposed = true; native = cachedRevision = cachedSource = undefined; ratio = null; },
  };
})
