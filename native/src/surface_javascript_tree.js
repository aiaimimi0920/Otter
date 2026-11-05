// 私有文档树：稳定 identity、live 集合、原子准入和有界移动；不提供外部宿主能力。
((fail, makeEvents) => {
  "use strict";
  // 不调用可被插件替换的 collection intrinsic，也不在私有数组上触发 species/iterator。
  const bind = fn => Function.prototype.call.bind(fn);
  const Weak = WeakMap, MapType = Map, ProxyType = Proxy, nullPrototype = Object.setPrototypeOf, create = Object.create;
  const array = () => nullPrototype([], null), push = (list, value) => { list[list.length] = value; };
  const map = (list, visit) => { const out = array(); for (let i = 0; i < list.length; i++) push(out, visit(list[i], i)); return out; };
  const filter = (list, match) => { const out = array(); for (let i = 0; i < list.length; i++) if (match(list[i])) push(out, list[i]); return out; };
  const indexOf = (list, value) => { for (let i = 0; i < list.length; i++) if (list[i] === value) return i; return -1; };
  const remove = (list, index) => { for (let i = index; i + 1 < list.length; i++) list[i] = list[i + 1]; list.length--; };
  const exec = bind(RegExp.prototype.exec), slice = bind(String.prototype.slice), index = bind(String.prototype.indexOf);
  // CSS 私有解析只分割固定字符串；不通过可变 RegExp Symbol.split/exec 调度。
  const split = (text, separator) => {
    const out = array(); let start = 0, next;
    while ((next = index(text, separator, start)) >= 0) { push(out, slice(text, start, next)); start = next + separator.length; }
    push(out, slice(text, start)); return out;
  };
  const words = text => {
    const out = array(); let start = 0;
    for (let i = 0; i <= text.length; i++) if (i === text.length || exec(/[\t\n\f\r ]/, text[i])) {
      if (i > start) push(out, slice(text, start, i)); start = i + 1;
    }
    return out;
  };
  const ops = Object.freeze({ array, push, map, filter, indexOf, remove,
    join: bind(Array.prototype.join), weakGet: bind(WeakMap.prototype.get), weakSet: bind(WeakMap.prototype.set),
    weakHas: bind(WeakMap.prototype.has), mapGet: bind(Map.prototype.get), mapSet: bind(Map.prototype.set),
    mapHas: bind(Map.prototype.has), mapDelete: bind(Map.prototype.delete), mapClear: bind(Map.prototype.clear),
    mapEntries: bind(Map.prototype.entries), mapNext: bind(Object.getPrototypeOf(new Map().entries()).next),
    mapSize: bind(Object.getOwnPropertyDescriptor(Map.prototype, "size").get),
    mapEach: bind(Map.prototype.forEach), setHas: bind(Set.prototype.has),
    Weak, MapType, ProxyType, string: String, freeze: Object.freeze, own: Object.hasOwn, apply: Reflect.apply,
    record: () => create(null), isArray: Array.isArray, number: Number, finite: Number.isFinite,
    float: parseFloat, abs: Math.abs, fround: Math.fround, exec, test: (pattern, text) => exec(pattern, text) !== null,
    trim: bind(String.prototype.trim), lower: bind(String.prototype.toLowerCase), slice, index, split, words });
  const state = new Weak(), nodes = new MapType();
  const events = makeEvents(ops, fail, node => ops.weakGet(state, node)?.parent ?? null);
  const containers = new Set(["div", "section", "article", "header", "footer", "main"]);
  const inline = new Set(["span", "strong", "em", "i", "b"]);
  const textTags = new Set([...inline, "p", "h1", "h2", "h3"]);
  const constructorKey = {};
  let created = 0, root, revision = create(null);
  // 私有 identity token 不回绕；几何仅持当前/上次 token，不保留 mutation 历史。
  const changed = () => { revision = create(null); };
  const get = node => ops.weakGet(state, node) ?? fail();
  const text = node => {
    const pending = array(), output = array(); push(pending, node);
    while (pending.length) {
      const value = get(pending[pending.length - 1]); pending.length--;
      if (value.tag === "#text") push(output, value.data);
      else for (let i = value.children.length - 1; i >= 0; i--) push(pending, value.children[i]);
    }
    return ops.join(output, "");
  };
  const collection = (node, elements) => {
    const items = () => filter(get(node).children, child => !elements || get(child).tag !== "#text");
    return new ProxyType(Object.create(null), {
      get(_, key) {
        if (key === "length") return items().length;
        if (key === "item") return index => items()[Number(index)] ?? null;
        if (key === Symbol.iterator) return function* () { for (let i = 0; i < items().length; i++) yield items()[i]; };
        if (typeof key === "string" && /^(0|[1-9][0-9]*)$/.test(key)) return items()[Number(key)];
      },
      set: fail, defineProperty: fail, deleteProperty: fail, setPrototypeOf: fail,
    });
  };
  const unlink = (node, notify = true) => {
    const s = get(node);
    if (s.parent) {
      if (notify) changed();
      const list = get(s.parent).children;
      remove(list, indexOf(list, node)); s.parent = null;
    }
  };
  const validate = (parent, child, moved) => {
    const p = get(parent), c = get(child);
    if (p.tag === "#text" || p.tag === "input" || child === root || c.tag === "#fragment") fail();
    let depth = 0;
    for (let node = parent; node; node = get(node).parent) {
      if (node === child) fail();
      if (get(node).tag !== "#fragment") depth++;
    }
    const pending = array(); push(pending, { node: child, level: depth + 1 });
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      if (item.level > 32) fail();
      const children = get(item.node).children;
      // 本批次移到 parent 的后代已成为兄弟，必须按最终树而不是旧子树检查深度。
      for (let i = 0; i < children.length; i++) if (indexOf(moved, children[i]) < 0) push(pending, { node: children[i], level: item.level + 1 });
    }
    if (!ops.setHas(containers, p.tag) && p.tag !== "#fragment" && c.tag !== "#text" && !ops.setHas(inline, c.tag)) fail();
  };
  const insert = (parent, values, replace, strings) => {
    // 参数、循环、深度和叶节点合同全部通过后才拆旧树；Fragment 本身不进入树。
    const next = array(), moved = array();
    const plan = child => {
      const old = indexOf(next, child);
      if (old >= 0) remove(next, old);
      push(next, child); if (indexOf(moved, child) < 0) push(moved, child);
    };
    for (let i = 0; i < values.length; i++) {
      let value = values[i];
      if (!ops.weakHas(state, value)) {
        if (!strings) fail();
        value = new Text(ops.string(value));
      }
      if (value === parent) fail();
      if (get(value).tag === "#fragment") {
        const children = get(value).children;
        for (let j = 0; j < children.length; j++) if (indexOf(moved, children[j]) < 0) plan(children[j]);
      } else plan(value);
    }
    for (let i = 0; i < next.length; i++) validate(parent, next[i], moved);
    changed();
    if (replace) while (get(parent).children.length) unlink(get(parent).children[0], false);
    for (let i = 0; i < next.length; i++) {
      const child = next[i];
      unlink(child, false); get(child).parent = parent; push(get(parent).children, child);
    }
  };
  class Node extends events.EventTarget {
    constructor(tag, key) {
      if (key !== constructorKey || ++created > 2048) fail();
      super();
      const s = nullPrototype({ tag, key: `dom:${created}`, data: "", parent: null, children: array(),
        attrs: new MapType(), childNodes: null, elements: null }, null);
      ops.weakSet(state, this, s); ops.mapSet(nodes, s.key, this);
      s.childNodes = collection(this, false); s.elements = collection(this, true);
      Object.seal(this);
    }
    get parentNode() { return get(this).parent; }
    get parentElement() { const parent = get(this).parent; return parent && get(parent).tag !== "#fragment" ? parent : null; }
    get childNodes() { return get(this).childNodes; }
    get firstChild() { return get(this).children[0] ?? null; }
    get lastChild() { const children = get(this).children; return children[children.length - 1] ?? null; }
    get nodeType() { return get(this).tag === "#text" ? 3 : get(this).tag === "#fragment" ? 11 : 1; }
    get textContent() { return text(this); }
    set textContent(value) {
      const s = get(this), data = ops.string(value ?? "");
      if (data.length > 65536) fail();
      if (s.tag === "#text") { changed(); s.data = data; }
      else insert(this, data ? [new Text(data)] : [], true, false);
    }
    appendChild(child) { insert(this, [child], false, false); return child; }
    append(...children) { insert(this, children, false, true); }
    replaceChildren(...children) { insert(this, children, true, true); }
    removeChild(child) {
      if (get(child).parent !== this) fail();
      unlink(child); return child;
    }
    remove() { unlink(this); }
    contains(other) {
      if (!ops.weakHas(state, other)) return false;
      for (let node = other; node; node = get(node).parent) if (node === this) return true;
      return false;
    }
  }
  class Element extends Node {
    constructor(tag) {
      tag = ops.string(tag).toLowerCase();
      if (!ops.setHas(containers, tag) && !ops.setHas(textTags, tag) && tag !== "button" && tag !== "canvas" && tag !== "input") fail();
      super(tag, constructorKey);
    }
    get tagName() { return get(this).tag.toUpperCase(); }
    get children() { return get(this).elements; }
    get firstElementChild() { return this.children[0] ?? null; }
    get lastElementChild() { return this.children[this.children.length - 1] ?? null; }
    get childElementCount() { return this.children.length; }
  }
  class HTMLElement extends Element {}
  class HTMLButtonElement extends HTMLElement {
    constructor() { super("button"); }
    get disabled() { return ops.mapHas(get(this).attrs, "disabled"); }
    set disabled(value) { if (value) this.setAttribute("disabled", ""); else this.removeAttribute("disabled"); }
  }
  class HTMLCanvasElement extends HTMLElement { constructor() { super("canvas"); } }
  class HTMLInputElement extends HTMLElement { constructor() { super("input"); } }
  class Text extends Node {
    constructor(value) { super("#text", constructorKey); this.textContent = value; }
    get data() { return get(this).data; }
    set data(value) { this.textContent = value; }
  }
  class DocumentFragment extends Node {
    constructor() { super("#fragment", constructorKey); }
    get children() { return get(this).elements; }
    get firstElementChild() { return this.children[0] ?? null; }
    get lastElementChild() { return this.children[this.children.length - 1] ?? null; }
  }
  const createElement = tag => {
    tag = ops.lower(ops.string(tag));
    return tag === "button" ? new HTMLButtonElement() : tag === "canvas" ? new HTMLCanvasElement() : tag === "input" ? new HTMLInputElement() : new HTMLElement(tag);
  };
  root = createElement("div");
  return { ops, nodes, get, text, containers, root, createElement, Text, DocumentFragment, changed, events, revision: () => revision,
    classes: { Event: events.Event, KeyboardEvent: events.KeyboardEvent, MouseEvent: events.MouseEvent, PointerEvent: events.PointerEvent, EventTarget: events.EventTarget, Node, Element, HTMLElement, HTMLButtonElement, HTMLCanvasElement, HTMLInputElement, Text, DocumentFragment } };
})
