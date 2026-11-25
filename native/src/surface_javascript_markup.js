// Strict fragment subset: construct a detached tree of the same type before swapping; not an HTML5 error-tolerant parser or a CSS engine.
((dom, fail, discard) => {
  "use strict";
  const op = dom.ops, entities = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };
  const { Element, HTMLCanvasElement } = dom.classes;
  const setAttribute = Element.prototype.setAttribute, setCanvasAttribute = HTMLCanvasElement.prototype.setAttribute;
  const parseInt = Number.parseInt, isInteger = Number.isInteger, fromCodePoint = String.fromCodePoint;
  const decode = value => {
    const pattern = /&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi;
    let match = op.exec(pattern, value), cursor = 0;
    if (!match) return value;
    const output = op.array();
    do {
      const token = match[1];
      op.push(output, op.slice(value, cursor, match.index));
      if (token[0] !== "#") op.push(output, op.own(entities, token) ? entities[token] : match[0]);
      else {
        const code = op.lower(token[1]) === "x" ? parseInt(op.slice(token, 2), 16) : op.number(op.slice(token, 1));
        if (!isInteger(code) || code < 1 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) fail();
        op.push(output, fromCodePoint(code));
      }
      cursor = pattern.lastIndex;
    } while ((match = op.exec(pattern, value)));
    op.push(output, op.slice(value, cursor));
    return op.join(output, "");
  };
  const starts = Function.prototype.call.bind(String.prototype.startsWith);
  const fragment = source => {
    const holder = dom.createStagedNode("#fragment"), stack = op.array(); op.push(stack, holder);
    let cursor = 0;
    const whitespace = () => { while (cursor < source.length && op.test(/[\t\n\f\r ]/, source[cursor])) cursor++; };
    while (cursor < source.length) {
      const parent = stack[stack.length - 1];
      if (starts(source, "<!--", cursor)) {
        const end = op.index(source, "-->", cursor + 4);
        if (end < 0) fail();
        cursor = end + 3; continue;
      }
      if (source[cursor] !== "<") {
        const next = op.index(source, "<", cursor), end = next < 0 ? source.length : next;
        dom.appendStaged(parent, dom.createStagedNode("#text", decode(op.slice(source, cursor, end)))); cursor = end; continue;
      }
      if (starts(source, "</", cursor)) {
        const match = op.exec(/^<\/([a-z][a-z0-9]*)[\t\n\f\r ]*>/i, op.slice(source, cursor));
        if (!match || stack.length === 1 || dom.get(parent).tag !== op.lower(match[1])) fail();
        stack.length--; cursor += match[0].length; continue;
      }
      const start = op.exec(/^<([a-z][a-z0-9]*)/i, op.slice(source, cursor));
      if (!start) fail();
      const tag = op.lower(start[1]), node = dom.createStagedNode(tag), seen = op.array();
      cursor += start[0].length;
      while (source[cursor] !== ">") {
        const before = cursor; whitespace();
        if (source[cursor] === ">") break;
        if (before === cursor) fail();
        const attribute = op.exec(/^[a-z_][a-z0-9_-]*/i, op.slice(source, cursor));
        if (!attribute) fail();
        const name = op.lower(attribute[0]);
        if (op.indexOf(seen, name) >= 0) fail();
        op.push(seen, name); cursor += attribute[0].length;
        const afterName = cursor; whitespace();
        let value = "";
        if (source[cursor] === "=") {
          cursor++; whitespace();
          const quote = source[cursor++];
          if (quote !== '"' && quote !== "'") fail();
          const end = op.index(source, quote, cursor);
          if (end < 0) fail();
          value = decode(op.slice(source, cursor, end)); cursor = end + 1;
        } else cursor = afterName;
        op.apply(tag === "canvas" ? setCanvasAttribute : setAttribute, node, [name, value]);
      }
      if (source[cursor] !== ">" || stack.length >= 32) fail();
      cursor++; dom.appendStaged(parent, node); if (tag !== "input") op.push(stack, node);
    }
    if (stack.length !== 1) fail();
    return holder;
  };
  // The original 256 KiB engine stack cannot accommodate nested Map callbacks and multiple RegExp.replace native calls.
  const escape = value => {
    const output = op.array();
    for (let i = 0; i < value.length; i++) {
      const c = value[i];
      op.push(output, c === "&" ? "&amp;" : c === "<" ? "&lt;" : c === ">" ? "&gt;" : c === '"' ? "&quot;" : c);
    }
    return op.join(output, "");
  };
  const serialize = children => {
    const pending = op.array(), output = op.array();
    for (let i = children.length - 1; i >= 0; i--) op.push(pending, children[i]);
    while (pending.length) {
      const node = pending[pending.length - 1]; pending.length--;
      if (typeof node === "string") { op.push(output, node); continue; }
      const s = dom.get(node);
      if (s.tag === "#text") { op.push(output, escape(s.data)); continue; }
      const attributes = op.array(), pairs = op.array();
      op.mapEach(s.attrs, (value, name) => op.push(pairs, [name, value]));
      for (let i = 0; i < pairs.length; i++) op.push(attributes, ` ${pairs[i][0]}="${escape(pairs[i][1])}"`);
      op.push(output, `<${s.tag}${op.join(attributes, "")}>`);
      if (s.tag === "input") continue;
      op.push(pending, `</${s.tag}>`);
      for (let i = s.children.length - 1; i >= 0; i--) op.push(pending, s.children[i]);
    }
    return op.join(output, "");
  };
  Object.defineProperty(Element.prototype, "innerHTML", {
    get() { return serialize(dom.get(this).children); },
    set(value) {
      const source = op.string(value);
      if (source.length > 65536 || op.index(source, "\0") >= 0) fail();
      dom.replaceMarkup(this, source, fragment, discard);
    },
  });
})
