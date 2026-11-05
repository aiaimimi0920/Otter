// 只接受已有原生 consumer 的值；未知声明拒绝，不以保存字符串冒充 CSS 生效。
((dom, fail, grid, spatial, fonts, makeBorders) => {
  "use strict";
  const op = dom.ops, definitions = new op.MapType();
  const define = (names, target, kind) => {
    const list = op.split(names, " ");
    for (let i = 0; i < list.length; i++) {
      const name = list[i], key = name.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
      op.mapSet(definitions, name, { target, key, kind });
    }
  };
  define("width height min-width min-height max-width max-height", "layout", "size");
  define("padding padding-top padding-right padding-bottom padding-left", "layout", "length");
  op.mapSet(grid.pairs, "padding", { names: op.words("padding-top padding-right padding-bottom padding-left"), separator: " ", quad: true });
  define(grid.names, "layout", "grid");
  define(spatial.names, "layout", "spatial");
  op.mapEach(spatial.pairs, (value, key) => op.mapSet(grid.pairs, key, value));
  define("border-radius", "style", "length");
  define("color background background-color", "style", "color");
  define("opacity", "style", "opacity");
  define(fonts.names + " font", "style", "font");
  op.mapSet(grid.pairs, "font", fonts.pair);
  define("flex-grow", "layout", "grow");
  const enums = new op.MapType();
  for (const [name, choices] of [
    ["display", "flex grid none"], ["flex-direction", "row column"], ["box-sizing", "border-box"],
    ["white-space", "normal nowrap pre pre-wrap"], ["text-align", "left center right"], ["text-overflow", "clip ellipsis"],
  ]) {
    op.mapSet(enums, name, op.split(choices, " "));
    define(name, name === "white-space" || op.index(name, "text-") === 0 ? "style" : "layout", "enum");
  }
  const color = value => {
    // 直接用同一捕获exec，避免输入转换重入时再挂一层test包装栈。
    if (value.length > 64 || !op.exec(/^[\x00-\x7F]*$/, value)) return false;
    if (op.exec(/^(transparent|currentcolor|black|white|red|green|blue|gray|grey|yellow|orange|purple|pink)$/, value)) return true;
    if (op.exec(/^#(?:[0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/, value)) return true;
    const match = op.exec(/^(rgb|rgba)\(([^()]*)\)$/, value);
    if (!match) return false;
    const parts = op.split(match[2], ","), count = match[1] === "rgb" ? 3 : 4;
    if (parts.length !== count) return false;
    let percentage;
    for (let i = 0; i < count; i++) {
      const item = op.trim(parts[i]);
      if (!op.exec(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)%?$/, item)) return false;
      const number = op.float(item);
      if (!op.finite(number) || op.abs(number) > 3.4028234e38) return false;
      if (i < 3) { const percent = item[item.length - 1] === "%"; if (i && percent !== percentage) return false; percentage = percent; }
    }
    return true;
  };
  const borders = makeBorders(dom, fail, color);
  define(borders.names, "style", "border");
  op.mapEach(borders.pairs, (value, key) => op.mapSet(grid.pairs, key, value));
  // 简单值校验与expand合一，不保留只由expand调用的额外validate栈。
  const expand = (name, value) => {
    const definition = op.mapGet(definitions, name);
    if (!definition) fail();
    if (definition.kind === "border") return borders.expand(name, value);
    if (definition.kind === "grid") {
      if (value.length > (op.index(name, "grid-template-") === 0 ? 4096 : 256) || !op.test(/^[\x00-\x7F]*$/, value)) fail();
      return grid.expand(name, op.lower(op.trim(value)));
    }
    if (definition.kind === "spatial") return spatial.expand(name, value);
    const items = op.array();
    if (definition.kind === "font") return fonts.expand(name, value);
    if (value.length > 256) fail();
    if (op.index(name, "padding") === 0 && !op.exec(/^[\x00-\x7F]*$/, value)) fail();
    value = op.lower(op.trim(value));
    const kind = definition.kind;
    if (kind === "enum") { if (op.indexOf(op.mapGet(enums, name), value) < 0) fail(); }
    else if (kind === "color") { if (!color(value)) fail(); }
    else if (kind === "size" || kind === "length") {
      if (kind === "size" && (name === "width" || name === "height") && value === "auto") { op.push(items, { name, value }); return items; }
      const pair = name === "padding" ? op.mapGet(grid.pairs, name) : undefined;
      const parts = pair ? op.words(value) : [value];
      if (!parts.length || parts.length > (pair ? 4 : 1)) fail();
      for (let i = 0; i < (pair ? 4 : 1); i++) {
        let item = parts[i] ?? parts[i === 3 ? 1 : 0] ?? parts[0];
        const match = op.exec(/^(0|(?:\d+(?:\.\d*)?|\.\d+)(px|%))$/, item);
        if (!match || (kind === "length" && match[2] === "%") || op.float(item) > (match[2] === "%" ? 1000 : 8192)) fail();
        // 所有尺寸都交原生 Length::decimal；不得让 .N/N. 被接受后回退成 auto/intrinsic。
        if (item[0] === ".") item = "0" + item;
        const suffix = match[2] ?? "";
        let number = op.slice(item, 0, item.length - suffix.length);
        if (number[number.length - 1] === ".") number = op.slice(number, 0, number.length - 1);
        item = number + suffix;
        if (item.length > 256) fail();
        op.push(items, { name: pair ? pair.names[i] : name, value: item });
      }
      return items;
    } else {
      if (!op.test(/^(?:\d+(?:\.\d*)?|\.\d+)$/, value)) fail();
      const number = op.number(value), maximum = kind === "opacity" ? 1 : 100;
      if (number > maximum) fail();
    }
    op.push(items, { name, value }); return items;
  };
  return { definitions, pairs: grid.pairs, expand, borders, lineHeight: fonts.projectLine,
    canonical: name => name === "background" ? "background-color" : name,
    inherited: ["color", "font-family", "font-size", "font-weight", "line-height", "white-space", "text-align"] };
})
