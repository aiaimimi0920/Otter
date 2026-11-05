// Inline 声明与 sheet 共用解析/优先级；先验证完整替换，再提交私有 Map。
((dom, fail, values) => {
  "use strict";
  const op = dom.ops, stores = new op.Weak(), views = new op.Weak();
  const strip = source => {
    if (source.length > 65536 || op.index(source, "\0") >= 0) fail();
    const output = op.array(); let quote = "", start = 0;
    for (let i = 0; i < source.length; i++) {
      const c = source[i];
      if (c === "\\") fail();
      if (quote) { if (c === quote) quote = ""; continue; }
      if (c === '"' || c === "'") quote = c;
      else if (c === "/" && source[i + 1] === "*") {
        const end = op.index(source, "*/", i + 2); if (end < 0) fail();
        op.push(output, op.slice(source, start, i)); op.push(output, " "); i = end + 1; start = i + 1;
      }
    }
    if (quote) fail(); op.push(output, op.slice(source, start)); return op.join(output, "");
  };
  const put = (store, name, value, priority, replace = false) => {
    name = values.canonical(name);
    if (priority !== "" && priority !== "important") fail();
    // 先验证整个 shorthand，再展开为独立 cascade key；不能让旧 longhand 越过后来的 reset。
    const items = values.expand(name, value);
    if (replace) dom.changed();
    for (let i = 0; i < items.length; i++) {
      const item = items[i], old = op.mapGet(store, item.name);
      if (replace || !old || priority === "important" || old.priority !== "important") op.mapSet(store, item.name, { value: item.value, priority });
    }
  };
  // 公开setter先完成String转换，私有parser只收字符串；重入几何时不叠加解析栈。
  const parse = source => {
    source = strip(source);
    const store = new op.MapType(), declarations = op.split(source, ";");
    if (declarations.length > 129) fail();
    for (let i = 0; i < declarations.length; i++) {
      const declaration = op.trim(declarations[i]); if (!declaration) continue;
      const colon = op.index(declaration, ":"); if (colon < 1) fail();
      const name = op.lower(op.trim(op.slice(declaration, 0, colon)));
      let value = op.trim(op.slice(declaration, colon + 1)), priority = "";
      const important = op.exec(/!\s*important\s*$/i, value);
      if (important) { priority = "important"; value = op.trim(op.slice(value, 0, important.index)); }
      put(store, name, value, priority);
    }
    return store;
  };
  const get = node => {
    dom.get(node);
    if (!op.weakHas(stores, node)) op.weakSet(stores, node, new op.MapType());
    return op.weakGet(stores, node);
  };
  const serialize = store => {
    const output = op.array();
    op.mapEach(store, (item, name) => op.push(output, name + ": " + item.value + (item.priority ? " !important" : "") + ";"));
    return op.join(output, " ");
  };
  const view = node => {
    const names = () => { const list = op.array(); op.mapEach(get(node), (_, name) => op.push(list, name)); return list; };
    const nameOf = key => {
      const output = op.array();
      for (let i = 0; i < key.length; i++) op.push(output, op.test(/[A-Z]/, key[i]) ? "-" + op.lower(key[i]) : key[i]);
      return op.join(output, "");
    };
    const methods = op.record();
    const canonical = name => values.canonical(op.lower(op.string(name)));
    const read = name => {
      name = canonical(name); const store = get(node), pair = op.mapGet(values.pairs, name);
      if (!pair) return op.mapGet(store, name);
      const first = op.mapGet(store, pair.names[0]), parts = op.array();
      if (!first) return undefined;
      for (let i = 0; i < pair.names.length; i++) {
        const item = op.mapGet(store, pair.names[i]);
        if (!item || item.priority !== first.priority) return undefined;
        op.push(parts, item.value);
      }
      if (op.own(pair, "border")) { const value = values.borders.reflect(name, parts); return value === undefined ? undefined : { value, priority: first.priority }; }
      if (name === "font") return { value: parts[0] + " " + parts[1] + "/" + parts[2] + " " + parts[3], priority: first.priority };
      if (op.own(pair, "quad") && pair.quad) { if (parts[1] === parts[3]) { parts.length--; if (parts[0] === parts[2]) parts.length--; } }
      if (parts.length === 2 && parts[0] === parts[1] && pair.separator === " ") parts.length--;
      return { value: op.join(parts, pair.separator), priority: first.priority };
    };
    methods.getPropertyValue = name => read(name)?.value ?? "";
    methods.getPropertyPriority = name => read(name)?.priority ?? "";
    methods.removeProperty = name => {
      name = canonical(name); const previous = methods.getPropertyValue(name);
      const pair = op.mapGet(values.pairs, name);
      dom.changed();
      if (pair) for (let i = 0; i < pair.names.length; i++) op.mapDelete(get(node), pair.names[i]);
      else op.mapDelete(get(node), name);
      return previous;
    };
    methods.setProperty = (name, value, priority = "") => {
      name = canonical(name); value = op.string(value); priority = op.lower(op.string(priority));
      if (value === "") { methods.removeProperty(name); return; }
      // setProperty 是指定单项替换，旧 important 不应阻止调用者显式覆盖。
      put(get(node), name, value, priority, true);
    };
    methods.item = index => names()[op.number(index)] ?? "";
    return new op.ProxyType(op.record(), {
      get(_, key) {
        if (key === "cssText") return serialize(get(node));
        if (key === "length") return op.mapSize(get(node));
        if (typeof key !== "string") return undefined;
        if (op.own(methods, key)) return methods[key];
        if (op.test(/^(0|[1-9][0-9]*)$/, key)) return methods.item(key);
        const name = nameOf(key); return op.mapHas(values.definitions, name) ? methods.getPropertyValue(name) : undefined;
      },
      set(_, key, value) {
        if (key === "cssText") { const next = parse(op.string(value)); dom.changed(); op.weakSet(stores, node, next); }
        // 与 cssText 相同，用户转换留在公开 setter；重入写入不携带外层 setProperty 栈。
        else { if (typeof key !== "string" || !op.mapHas(values.definitions, nameOf(key))) fail(); methods.setProperty(nameOf(key), op.string(value)); }
        return true;
      },
      defineProperty: fail, deleteProperty: fail, setPrototypeOf: fail,
    });
  };
  Object.defineProperty(dom.classes.Element.prototype, "style", {
    get() { if (!op.weakHas(views, this)) op.weakSet(views, this, view(this)); return op.weakGet(views, this); },
    set(value) { const next = parse(op.string(value)); dom.changed(); op.weakSet(stores, this, next); },
  });
  return { parse, strip, get };
})
