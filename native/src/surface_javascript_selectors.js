// Bounded simple selectors share one matcher; unsupported CSS combinators must not default to a match or an empty result.
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
    // result is this query's exclusively owned node snapshot, not the tree's children; restoring the public Array interface needs no further copy.
    // Use no prototypes while populating, including on descriptors, so inherited setters/getters cannot contaminate static collections.
    const item = op.record();
    item.value = index => {
      // Plugins may replace Number; one ToPropertyKey reads only own data entries, never inherited numeric properties.
      const entry = describe(result, Number(index));
      return entry ? entry.value ?? null : null;
    };
    define(result, "item", item);
    return op.freeze(setPrototype(result, arrayPrototype));
  };
  // Single-result queries stop at the first match instead of collecting, copying, and freezing every match.
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
