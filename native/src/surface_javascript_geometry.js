// Synchronous border/client boxes use the same native measurements; explicitly fail rather than guess without a viewport or separate inline run.
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
    // Existing projection flattens inline content inside leaves, leaving no separate box; do not substitute leaf/parent rectangles for its dimensions.
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
    // Skip only repeated projection within the same operation; every read still passes native reads/layout/deadline admission.
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
