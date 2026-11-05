// 文本 input 的值/dirty 反射 owner；沿同一 DOM/事件/投影，不复制 HWND 草稿或远端权限。
((dom, fail) => {
  "use strict";
  const op = dom.ops, values = new op.Weak(), prototype = dom.classes.HTMLInputElement.prototype;
  const define = Object.defineProperty, code = Function.prototype.call.bind(String.prototype.charCodeAt);
  const state = node => { const value = dom.get(node); if (value.tag !== "input") fail(); return value; };
  const split = Function.prototype.call.bind(String.prototype.split);
  // 私有无原型分隔符不查插件的 Symbol.split/toPrimitive；split 创建连续自有数据槽。
  const separator = text => op.freeze(define(Object.create(null), "toString", { value: () => text }));
  const cr = separator("\r"), lf = separator("\n");
  const compact = raw => {
    if (!op.exec(/[\r\n]/, raw)) {
      if (raw.length > 48 * 1024) fail();
      return raw;
    }
    // 两次 native 扫描避免交替换行产生32768次 JS/RegExp 往返；raw 仍至多65536。
    const output = op.join(split(op.join(split(raw, cr), ""), lf), "");
    if (output.length > 48 * 1024) fail();
    return output;
  };
  const sanitized = raw => {
    if (raw.length > 65536) fail();
    // 常见 ASCII 不建逐字符数组；捕获的 exec 不经过插件可覆盖的 RegExp 方法。
    if (!op.exec(/[\0\u0080-\uFFFF]/, raw)) return compact(raw);
    let bytes = 0;
    for (let i = 0; i < raw.length; i++) {
      const c = code(raw, i);
      if (c === 10 || c === 13) continue;
      if (!c || (c >= 0xDC00 && c <= 0xDFFF)) fail();
      if (c >= 0xD800 && c <= 0xDBFF) {
        const next = code(raw, i + 1);
        if (!(next >= 0xDC00 && next <= 0xDFFF)) fail();
        i++; bytes += 4;
      } else { bytes += c < 128 ? 1 : c < 2048 ? 2 : 3; }
      if (bytes > 48 * 1024) fail();
    }
    // 完整验证后再去换行，不能把换行两侧的孤立代理项错误拼成合法字符。
    return compact(raw);
  };
  const value = node => op.weakHas(values, node) ? op.weakGet(values, node) : sanitized(op.mapGet(state(node).attrs, "value") ?? "");
  const write = (node, raw) => {
    state(node); const next = sanitized(raw);
    dom.changed(); op.weakSet(values, node, next);
  };
  define(prototype, "value", { configurable: true, enumerable: true, get() { state(this); return value(this); }, set(raw) { write(this, op.string(raw)); } });
  for (const [property, attribute] of [["defaultValue", "value"], ["placeholder", "placeholder"]]) {
    define(prototype, property, { configurable: true, enumerable: true, get() { return op.mapGet(state(this).attrs, attribute) ?? ""; }, set(raw) { this.setAttribute(attribute, raw); } });
  }
  define(prototype, "type", { configurable: true, enumerable: true, get() { state(this); return "text"; }, set(raw) { this.setAttribute("type", raw); } });
  define(prototype, "disabled", { configurable: true, enumerable: true, get() { return op.mapHas(state(this).attrs, "disabled"); }, set(raw) { if (raw) this.setAttribute("disabled", ""); else this.removeAttribute("disabled"); } });
  const target = id => {
    const node = op.mapGet(dom.nodes, id);
    if (!node || op.mapHas(state(node).attrs, "disabled")) fail();
    let current = node;
    while (current && current !== dom.root) current = dom.get(current).parent;
    if (current !== dom.root) fail();
    return node;
  };
  return {
    attribute(name, raw) {
      if (name === "disabled" || name === "placeholder") return;
      if (name === "type" && (raw === "" || op.lower(raw) === "text")) return;
      if (name === "value") { sanitized(raw); return; }
      fail();
    },
    props(node) {
      const attrs = state(node).attrs, props = op.record();
      props.value = value(node); props.disabled = op.mapHas(attrs, "disabled");
      props.placeholder = op.mapGet(attrs, "placeholder") ?? "";
      return props;
    },
    target,
    accept(input) {
      const node = target(input.target);
      if (typeof input.value !== "string") fail();
      write(node, input.value); return node;
    },
  };
})
