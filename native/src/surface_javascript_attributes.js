// 元数据属性统一反射；不接受 CSS、URL、事件代码或未实现的表单语义。
((dom, fail, form) => {
  "use strict";
  const { Element } = dom.classes, get = dom.get, op = dom.ops;
  const entries = node => { const values = op.array(); op.mapEach(get(node).attrs, (value, name) => op.push(values, [name, value])); return values; };
  const nameOf = value => {
    const name = op.string(value).toLowerCase();
    if (name.length > 128 || !/^[a-z_][a-z0-9_-]*$/.test(name)) fail();
    return name;
  };
  const allowed = (node, name, value) => {
    if (op.indexOf(["id", "class", "title", "role"], name) >= 0 || /^(data|aria)-[a-z_][a-z0-9_-]*$/.test(name)) return;
    if (get(node).tag === "button" && (name === "disabled" || (name === "type" && value === "button"))) return;
    if (get(node).tag === "input") { form.attribute(name, value); return; }
    fail();
  };
  Object.assign(Element.prototype, {
    setAttribute(name, value) {
      name = nameOf(name); value = op.string(value);
      if (value.length > 4096) fail();
      allowed(this, name, value);
      const attrs = get(this).attrs;
      if (!op.mapHas(attrs, name) && op.mapSize(attrs) >= 64) fail();
      dom.changed();
      op.mapSet(attrs, name, value);
    },
    getAttribute(name) { return op.mapGet(get(this).attrs, nameOf(name)) ?? null; },
    hasAttribute(name) { return op.mapHas(get(this).attrs, nameOf(name)); },
    removeAttribute(name) { const attrs = get(this).attrs; name = nameOf(name); dom.changed(); op.mapDelete(attrs, name); },
    getAttributeNames() { return Array.from(op.map(entries(this), pair => pair[0])); },
  });
  const dataset = node => {
    const toAttribute = key => {
      if (typeof key !== "string" || !key || /-[a-z]/.test(key)) fail();
      return nameOf("data-" + key.replace(/[A-Z]/g, c => "-" + c.toLowerCase()));
    };
    const keys = () => op.map(op.filter(entries(node), pair => pair[0].startsWith("data-")),
      pair => pair[0].slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase()));
    return new op.ProxyType(Object.create(null), {
      get(_, key) { return typeof key === "symbol" ? undefined : node.getAttribute(toAttribute(key)) ?? undefined; },
      set(_, key, value) { node.setAttribute(toAttribute(key), value); return true; },
      deleteProperty(_, key) { node.removeAttribute(toAttribute(key)); return true; },
      has(_, key) { return typeof key === "string" && node.hasAttribute(toAttribute(key)); },
      ownKeys: () => keys(),
      getOwnPropertyDescriptor(_, key) {
        if (typeof key === "string" && op.indexOf(keys(), key) >= 0) return {
          configurable: true, enumerable: true, writable: true, value: node.getAttribute(toAttribute(key)),
        };
      },
      defineProperty: fail, setPrototypeOf: fail,
    });
  };
  const classList = node => {
    const unique = values => { const out = op.array(); for (let i = 0; i < values.length; i++) if (values[i] && op.indexOf(out, values[i]) < 0) op.push(out, values[i]); return out; };
    const tokens = () => unique((node.getAttribute("class") ?? "").split(/[\t\n\f\r ]+/));
    const check = values => op.map(values, value => {
      value = op.string(value);
      if (!value || /[\t\n\f\r ]/.test(value)) fail();
      return value;
    });
    const write = values => node.setAttribute("class", op.join(values, " "));
    return op.freeze({
      get length() { return tokens().length; },
      get value() { return node.className; },
      contains(token) { return op.indexOf(tokens(), check([token])[0]) >= 0; },
      add(...values) { const add = check(values), out = tokens(); for (let i = 0; i < add.length; i++) op.push(out, add[i]); write(unique(out)); },
      remove(...values) { const remove = check(values); write(op.filter(tokens(), value => op.indexOf(remove, value) < 0)); },
      toggle(token, force) {
        token = check([token])[0];
        const values = tokens(), present = op.indexOf(values, token) >= 0;
        const wanted = force === undefined ? !present : !!force;
        if (wanted !== present) { if (wanted) op.push(values, token); write(wanted ? values : op.filter(values, value => value !== token)); }
        return wanted;
      },
      item(index) { return tokens()[Number(index)] ?? null; },
      toString() { return node.className; },
      *[Symbol.iterator]() { const values = tokens(); for (let i = 0; i < values.length; i++) yield values[i]; },
    });
  };
  const datasets = new op.Weak(), lists = new op.Weak();
  for (const [property, attribute] of [["id", "id"], ["className", "class"], ["title", "title"]]) {
    Object.defineProperty(Element.prototype, property, {
      get() { return this.getAttribute(attribute) ?? ""; },
      set(value) { this.setAttribute(attribute, value); },
    });
  }
  Object.defineProperties(Element.prototype, {
    dataset: { get() { if (!op.weakHas(datasets, this)) op.weakSet(datasets, this, dataset(this)); return op.weakGet(datasets, this); } },
    classList: { get() { if (!op.weakHas(lists, this)) op.weakSet(lists, this, classList(this)); return op.weakGet(lists, this); } },
  });
})
