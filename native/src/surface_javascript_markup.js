// 严格 fragment 子集：先构造脱离的同型树再交换；不是 HTML5 容错 parser 或 CSS 引擎。
((dom, fail) => {
  "use strict";
  const op = dom.ops, entities = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };
  const decode = value => value.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (raw, token) => {
    if (token[0] !== "#") return op.own(entities, token) ? entities[token] : raw;
    const code = token[1].toLowerCase() === "x" ? Number.parseInt(token.slice(2), 16) : Number(token.slice(1));
    if (!Number.isInteger(code) || code < 1 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) fail();
    return String.fromCodePoint(code);
  });
  const fragment = input => {
    const source = op.string(input), holder = new dom.DocumentFragment(), stack = op.array(); op.push(stack, holder);
    if (source.length > 65536 || source.includes("\0")) fail();
    let cursor = 0;
    const whitespace = () => { while (/[\t\n\f\r ]/.test(source[cursor] ?? "") && cursor < source.length) cursor++; };
    while (cursor < source.length) {
      const parent = stack[stack.length - 1];
      if (source.startsWith("<!--", cursor)) {
        const end = source.indexOf("-->", cursor + 4);
        if (end < 0) fail();
        cursor = end + 3; continue;
      }
      if (source[cursor] !== "<") {
        const next = source.indexOf("<", cursor), end = next < 0 ? source.length : next;
        parent.appendChild(new dom.Text(decode(source.slice(cursor, end)))); cursor = end; continue;
      }
      if (source.startsWith("</", cursor)) {
        const match = /^<\/([a-z][a-z0-9]*)[\t\n\f\r ]*>/i.exec(source.slice(cursor));
        if (!match || stack.length === 1 || dom.get(parent).tag !== match[1].toLowerCase()) fail();
        stack.length--; cursor += match[0].length; continue;
      }
      const start = /^<([a-z][a-z0-9]*)/i.exec(source.slice(cursor));
      if (!start) fail();
      const node = dom.createElement(start[1]), seen = op.array();
      cursor += start[0].length;
      while (source[cursor] !== ">") {
        const before = cursor; whitespace();
        if (source[cursor] === ">") break;
        if (before === cursor) fail();
        const attribute = /^[a-z_][a-z0-9_-]*/i.exec(source.slice(cursor));
        if (!attribute) fail();
        const name = attribute[0].toLowerCase();
        if (op.indexOf(seen, name) >= 0) fail();
        op.push(seen, name); cursor += attribute[0].length;
        const afterName = cursor; whitespace();
        let value = "";
        if (source[cursor] === "=") {
          cursor++; whitespace();
          const quote = source[cursor++];
          if (quote !== '"' && quote !== "'") fail();
          const end = source.indexOf(quote, cursor);
          if (end < 0) fail();
          value = decode(source.slice(cursor, end)); cursor = end + 1;
        } else cursor = afterName;
        node.setAttribute(name, value);
      }
      if (source[cursor] !== ">" || stack.length >= 32) fail();
      cursor++; parent.appendChild(node); if (dom.get(node).tag !== "input") op.push(stack, node);
    }
    if (stack.length !== 1) fail();
    return holder;
  };
  // 原 256 KiB 引擎栈下不能叠加 Map callback 和多层 RegExp.replace native 调用。
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
  Object.defineProperty(dom.classes.Element.prototype, "innerHTML", {
    get() { return serialize(dom.get(this).children); },
    set(value) { this.replaceChildren(fragment(value)); },
  });
})
