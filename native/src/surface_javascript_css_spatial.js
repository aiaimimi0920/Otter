// 定位/裁剪只接已有原生 owner；直接包含块和绘制顺序不满足子集时明确拒绝。
((dom, fail) => {
  "use strict";
  const op = dom.ops, pairs = new op.MapType(), sides = op.words("top right bottom left");
  op.mapSet(pairs, "inset", { names: sides, separator: " ", quad: true });
  op.mapSet(pairs, "overflow", { names: op.words("overflow-x overflow-y"), separator: " " });
  const names = "position top right bottom left inset overflow overflow-x overflow-y";
  const expand = (name, value) => {
    if (value.length > 256 || !op.test(/^[\x00-\x7F]*$/, value)) fail();
    value = op.lower(op.trim(value));
    const pair = op.mapGet(pairs, name), parts = pair ? op.words(value) : op.array(), output = op.array();
    if (!pair) op.push(parts, value);
    if (!parts.length || parts.length > (pair ? pair.names.length : 1)) fail();
    for (let i = 0; i < (pair ? pair.names.length : 1); i++) {
      const key = pair ? pair.names[i] : name;
      let item = parts[i] ?? parts[i === 3 ? 1 : 0] ?? parts[0];
      if (key === "position") { if (op.indexOf(["static", "relative", "absolute"], item) < 0) fail(); }
      else if (op.index(key, "overflow-") === 0) { if (item !== "visible" && item !== "hidden") fail(); }
      else if (item !== "auto") {
        const match = op.exec(/^(0|(?:\d+(?:\.\d*)?|\.\d+)(px|%))$/, item);
        if (!match || op.float(item) > (match[2] === "%" ? 1000 : 8192)) fail();
        // 原生 decimal 不接受 .N/N.；规范化而非让非法长度静默落回默认值。
        if (item[0] === ".") item = "0" + item;
        item = op.slice(item, 0, item.length - (match[2]?.length ?? 0));
        if (item[item.length - 1] === ".") item = op.slice(item, 0, item.length - 1);
        item += match[2] ?? "";
        if (item.length > 256) fail();
      }
      op.push(output, { name: key, value: item });
    }
    return output;
  };
  const positioned = css => css?.position === "relative" || css?.position === "absolute";
  const validate = (node, css, parent) => {
    const state = dom.get(node), absolute = css.position === "absolute";
    const anchored = key => op.own(css, key) && css[key] !== "auto";
    const container = op.setHas(dom.containers, state.tag);
    const x = css["overflow-x"] ?? "visible", y = css["overflow-y"] ?? "visible";
    const text = !container && op.indexOf(["canvas", "input", "button", "#text"], state.tag) < 0;
    // 混合轴会按 CSS 计算成 auto；未接通 JS 滚动状态前不得意外开放滚动容器。
    if (x !== y || (!container && (op.own(css, "overflow-x") || op.own(css, "overflow-y")) && (!text || x !== "hidden"))) fail();
    // 省略只接文字叶的水平单行 consumer；不继承/截断字符串或把未实现的 inline flow 当成支持。
    if (op.own(css, "text-overflow") && (!text || (css["text-overflow"] === "ellipsis" &&
        (x !== "hidden" || css["white-space"] !== "nowrap" || (css["text-align"] ?? "left") !== "left")))) fail();
    for (let i = 0; i < sides.length; i++) if (anchored(sides[i]) && !positioned(css)) fail();
    if (positioned(css) && state.parent && !positioned(parent) && (absolute || state.parent !== dom.root)) fail();
    // 尚无 CSS definite-height 分析；非根 relative 的纵向百分比不能按最终父高猜算。
    if (css.position === "relative" && state.parent)
      for (let i = 0; i < 2; i++) { const value = css[i ? "bottom" : "top"]; if (value && op.index(value, "%") >= 0 && op.float(value) !== 0) fail(); }
    if (!absolute) return;
    if (parent?.display === "grid" || op.own(css, "flex-grow") || op.own(css, "align-self") || op.own(css, "justify-self")) fail();
    for (let i = 0; i < 2; i++) {
      const first = anchored(i ? "top" : "left"), last = anchored(i ? "bottom" : "right");
      const size = css[i ? "height" : "width"];
      if ((!first && !last) || ((!first || !last) && (!size || size === "auto"))) fail();
    }
  };
  const order = (node, computed) => {
    if (op.weakGet(computed, node).display === "none") return;
    const children = dom.get(node).children; let tail = false;
    for (let i = 0; i < children.length; i++) {
      const state = dom.get(children[i]), css = op.weakGet(computed, children[i]);
      if (css.display === "none" || (state.tag === "#text" && !op.test(/\S/, state.data))) continue;
      if (positioned(css)) tail = true;
      else if (tail) fail();
    }
  };
  return { names, pairs, expand, validate, order };
})
