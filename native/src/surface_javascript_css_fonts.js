// 字体声明唯一准入/展开 owner；不查询字体或布局，native Engine 仍负责实际度量。
((dom, fail) => {
  "use strict";
  const op = dom.ops, names = "font-weight font-size line-height font-family";
  const bounded = (value, maximum) => {
    if (!value || value.length > maximum || !op.exec(/^[\x00-\x7F\u3400-\u9fff]*$/, value)) fail();
    let bytes = value.length;
    for (let i = 0; i < value.length; i++) if (op.exec(/[\u3400-\u9fff]/, value[i])) bytes += 2;
    if (bytes > maximum) fail();
  };
  const family = value => {
    bounded(value, 256);
    const parts = op.split(value, ","), output = op.array();
    if (parts.length > 8) fail();
    for (let i = 0; i < parts.length; i++) {
      const part = op.trim(parts[i]), quote = part[0] === "'" || part[0] === '"';
      const name = quote ? op.slice(part, 1, part.length - 1) : op.join(op.words(part), " ");
      if (!name || name.length > 31 || !op.exec(/^[A-Za-z0-9 _\-\u3400-\u9fff]+$/, name) || (quote && part[part.length - 1] !== part[0])) fail();
      const lower = op.lower(name);
      if (!quote && (op.exec(/^(inherit|initial|unset|revert|revert-layer|default|cursive|fantasy|ui-serif|ui-sans-serif|ui-monospace|ui-rounded|emoji|math|fangsong)$/, lower) || !op.exec(/^[A-Za-z_\u3400-\u9fff][A-Za-z0-9_\-\u3400-\u9fff]*(?: [A-Za-z_\u3400-\u9fff][A-Za-z0-9_\-\u3400-\u9fff]*)*$/, name))) fail();
      op.push(output, !quote && op.exec(/^(sans-serif|serif|monospace|system-ui)$/, lower) ? lower : '"' + name + '"');
    }
    const result = op.join(output, ","); bounded(result, 256); return result;
  };
  const scalar = (value, maximum, pixels) => {
    const pattern = pixels ? /^(?:\d+(?:\.\d*)?|\.\d+)px$/ : /^(?:\d+(?:\.\d*)?|\.\d+)$/;
    if (!op.exec(pattern, value)) fail();
    const number = op.float(value);
    if (!op.finite(number) || number < 1 || number > maximum) fail();
    return op.string(number) + (pixels ? "px" : "");
  };
  const weight = value => {
    if (value === "normal" || value === "bold") return value === "normal" ? "400" : "700";
    if (!op.exec(/^\d+$/, value) || op.number(value) < 100 || op.number(value) > 900) fail();
    return op.string(op.number(value));
  };
  const line = value => value === "normal" ? value : scalar(value, op.index(value, "px") >= 0 ? 8192 : 1000, op.index(value, "px") >= 0);
  const expand = (name, source) => {
    const items = op.array();
    if (name === "font-family") { op.push(items, {name, value: family(source)}); return items; }
    bounded(source, name === "font" ? 512 : 256);
    if (name !== "font") {
      const value = op.lower(op.trim(source));
      op.push(items, {name, value: name === "font-weight" ? weight(value) : name === "font-size" ? scalar(value, 8192, true) : line(value)});
      return items;
    }
    // 只接收有真实 consumer 的 normal 样式；先完成四个 longhand 再让 declarations 提交。
    const match = op.exec(/^(?:(normal|bold|\d+)\s+)?((?:\d+(?:\.\d*)?|\.\d+)px)(?:\s*\/\s*(normal|(?:\d+(?:\.\d*)?|\.\d+)(?:px)?))?\s+(.+)$/i, op.trim(source));
    if (!match) fail();
    const parts = [weight(op.lower(match[1] ?? "normal")), scalar(op.lower(match[2]), 8192, true), line(op.lower(match[3] ?? "normal")), family(match[4])];
    const keys = op.split(names, " ");
    for (let i = 0; i < keys.length; i++) op.push(items, {name: keys[i], value: parts[i]});
    return items;
  };
  const projectLine = css => {
    const value = css["line-height"];
    if (value === undefined || value === "normal") return value;
    if (op.index(value, "px") >= 0) {
      // 保留旧 native 非重叠行高合同；已计算 px 在后代仍是同一个绝对值。
      if (!css["font-size"] || op.float(value) < op.float(css["font-size"])) fail();
      return value;
    }
    // CSS unitless 继承数字而非父像素；每个投影节点交原 em resolver 按本节点字号求值。
    // 无显式字号时使用原 native 初始字体，1000 倍仍在原 32768 高度门以内。
    if (css["font-size"] && op.fround(op.fround(op.float(value)) * op.fround(op.float(css["font-size"]))) > 32768) fail();
    return value + "em";
  };
  return { names, expand, projectLine, pair: {names: op.split(names, " ")} };
})
