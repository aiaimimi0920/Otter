// 有界简单选择器共用匹配器；不把未支持的 CSS 组合器默认为匹配或空结果。
((dom, fail) => {
  "use strict";
  const { Element, DocumentFragment } = dom.classes, op = dom.ops;
  const define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor;
  const setPrototype = Object.setPrototypeOf, arrayPrototype = Array.prototype;
  const compile = value => {
    const selector = op.string(value).trim();
    if (!selector || selector.length > 256) fail();
    if (selector === "*") return () => true;
    if (/^[a-z][a-z0-9-]*$/i.test(selector)) return node => node.tagName.toLowerCase() === selector.toLowerCase();
    if (/^[.#][a-z_][a-z0-9_-]*$/i.test(selector)) {
      return selector[0] === "#" ? node => node.id === selector.slice(1) : node => node.classList.contains(selector.slice(1));
    }
    const attr = /^\[([a-z_][a-z0-9_-]*)(?:=(?:"([^"\r\n]*)"|'([^'\r\n]*)'))?\]$/i.exec(selector);
    if (!attr || attr[1].length > 128) fail();
    const name = attr[1].toLowerCase(), expected = attr[2] ?? attr[3];
    return expected === undefined ? node => node.hasAttribute(name) : node => node.getAttribute(name) === expected;
  };
  const search = (node, selector, includeRoot, firstOnly) => {
    const matches = compile(selector), result = firstOnly ? null : op.array();
    if (includeRoot && matches(node)) {
      if (firstOnly) return node;
      op.push(result, node);
    }
    const pending = op.array(), initial = dom.get(node).children;
    for (let i = initial.length - 1; i >= 0; i--) op.push(pending, initial[i]);
    while (pending.length) {
      const child = pending[pending.length - 1]; pending.length--;
      const state = dom.get(child);
      if (state.tag === "#text") continue;
      if (matches(child)) {
        if (firstOnly) return child;
        op.push(result, child);
      }
      for (let i = state.children.length - 1; i >= 0; i--) op.push(pending, state.children[i]);
    }
    if (firstOnly) return null;
    // result 是本次查询独占的节点快照，不是树的 children；恢复公开 Array 接口无需再复制。
    // 填充时无原型，descriptor 也无原型，避免继承 setter/getter 污染静态集合。
    const item = op.record();
    item.value = index => {
      // Number 可由插件改写；一次 ToPropertyKey 只取自有数据项，不读取继承的数字属性。
      const entry = describe(result, Number(index));
      return entry ? entry.value ?? null : null;
    };
    define(result, "item", item);
    return op.freeze(setPrototype(result, arrayPrototype));
  };
  // 单结果查询在首次命中后停止，不再收集、复制和冻结全部匹配项。
  const query = (node, selector, includeRoot = false) => search(node, selector, includeRoot, false);
  const first = (node, selector, includeRoot = false) => search(node, selector, includeRoot, true);
  for (const prototype of [Element.prototype, DocumentFragment.prototype]) Object.assign(prototype, {
    querySelectorAll(selector) { return query(this, selector); },
    querySelector(selector) { return first(this, selector); },
  });
  Object.assign(Element.prototype, {
    matches(selector) { return compile(selector)(this); },
    closest(selector) {
      const matches = compile(selector);
      for (let node = this; node; node = node.parentElement) if (matches(node)) return node;
      return null;
    },
  });
  return { query, first };
})
