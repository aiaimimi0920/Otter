// 有界简单选择器共用匹配器；不把未支持的 CSS 组合器默认为匹配或空结果。
((dom, fail) => {
  "use strict";
  const { Element, DocumentFragment } = dom.classes, op = dom.ops;
  const compile = value => {
    const selector = op.string(value).trim();
    if (!selector || selector.length > 256) fail();
    if (selector === "*") return () => true;
    if (/^[a-z][a-z0-9-]*$/i.test(selector)) return node => node.tagName.toLowerCase() === selector.toLowerCase();
    if (/^[.#][a-z_][a-z0-9_-]*$/i.test(selector)) {
      return selector[0] === "#" ? node => node.id === selector.slice(1) : node => node.classList.contains(selector.slice(1));
    }
    const attr = /^\[([a-z_][a-z0-9_-]*)(?:=(?:"([^"\r\n]*)"|'([^'\r\n]*)'))?\]$/i.exec(selector);
    if (!attr) fail();
    const name = attr[1].toLowerCase(), expected = attr[2] ?? attr[3];
    return expected === undefined ? node => node.hasAttribute(name) : node => node.getAttribute(name) === expected;
  };
  const query = (node, selector, includeRoot = false) => {
    const matches = compile(selector), result = op.array();
    if (includeRoot && matches(node)) op.push(result, node);
    const pending = op.array(), initial = dom.get(node).children;
    for (let i = initial.length - 1; i >= 0; i--) op.push(pending, initial[i]);
    while (pending.length) {
      const child = pending[pending.length - 1]; pending.length--;
      const state = dom.get(child);
      if (state.tag === "#text") continue;
      if (matches(child)) op.push(result, child);
      for (let i = state.children.length - 1; i >= 0; i--) op.push(pending, state.children[i]);
    }
    // 静态集合与 live children 分离；防止调用者改结果影响真实树。
    // 返回值不包含私有数组，恢复普通静态 Array 接口供 Array.from/迭代/forEach 使用。
    const output = [];
    for (let i = 0; i < result.length; i++) output[i] = result[i];
    Object.defineProperty(output, "item", { value: index => output[Number(index)] ?? null });
    return op.freeze(output);
  };
  for (const prototype of [Element.prototype, DocumentFragment.prototype]) Object.assign(prototype, {
    querySelectorAll(selector) { return query(this, selector); },
    querySelector(selector) { return query(this, selector)[0] ?? null; },
  });
  Object.assign(Element.prototype, {
    matches(selector) { return compile(selector)(this); },
    closest(selector) {
      const matches = compile(selector);
      for (let node = this; node; node = node.parentElement) if (matches(node)) return node;
      return null;
    },
  });
  return { query };
})
