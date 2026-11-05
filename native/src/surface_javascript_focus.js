// 仅记录当前 operation 的有界焦点意图；实际键盘焦点/草稿仍由宿主唯一 HWND owner 决定。
((dom, fail) => {
  "use strict";
  const op = dom.ops, define = Object.defineProperty;
  let requests = op.array(), enabled = false;
  const request = (node, focus, options) => {
    const state = dom.get(node);
    if (state.tag !== "input" || options !== undefined) fail();
    if (!enabled || op.mapHas(state.attrs, "disabled")) return;
    let current = node;
    while (current && current !== dom.root) current = dom.get(current).parent;
    if (current !== dom.root) return;
    if (requests.length >= 32) fail();
    const value = op.record(); value.target = state.key; value.focus = focus;
    op.push(requests, value);
  };
  define(dom.classes.HTMLInputElement.prototype, "focus", {
    configurable: true, writable: true, value(options) { request(this, true, options); },
  });
  define(dom.classes.HTMLInputElement.prototype, "blur", {
    configurable: true, writable: true, value() { request(this, false); },
  });
  return {
    begin(active) { requests = op.array(); enabled = active; },
    collect: () => requests,
  };
})
