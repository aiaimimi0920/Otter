// 实心四边框准入/简写展开；只投影已有原生四边 owner，不接受其他线型或 border-image。
((dom, fail, color) => {
  "use strict";
  const op = dom.ops, sides = op.words("top right bottom left"), titles = op.words("Top Right Bottom Left"), pairs = new op.MapType();
  const names = op.array();
  const widths = op.array(), styles = op.array(), colors = op.array(), all = op.array();
  for (let i = 0; i < 4; i++) {
    const base = "border-" + sides[i], keys = [base + "-width", base + "-style", base + "-color"];
    op.push(names, base); op.mapSet(pairs, base, { names: keys, separator: " ", border: true });
    for (let k = 0; k < 3; k++) { op.push(names, keys[k]); op.push(all, keys[k]); }
    op.push(widths, keys[0]); op.push(styles, keys[1]); op.push(colors, keys[2]);
  }
  for (const [name, keys] of [["border-width", widths], ["border-style", styles], ["border-color", colors]]) {
    op.push(names, name); op.mapSet(pairs, name, { names: keys, separator: " ", quad: true });
  }
  op.push(names, "border"); op.mapSet(pairs, "border", { names: all, separator: " ", border: true });
  const tokens = value => {
    if (value.length > 512 || !op.test(/^[\x00-\x7F]*$/, value)) fail();
    const output = op.array(); let depth = 0, start = 0;
    for (let i = 0; i <= value.length; i++) {
      const c = value[i];
      if (c === "(") { if (depth) fail(); depth++; }
      if (c === ")") { if (!depth) fail(); depth--; }
      if (i === value.length || (!depth && op.test(/\s/, c))) {
        if (i > start) op.push(output, op.slice(value, start, i)); start = i + 1;
      }
    }
    if (depth || !output.length || output.length > 4) fail();
    return output;
  };
  const width = value => {
    if (value === "thin") return "1px";
    if (value === "medium") return "3px";
    if (value === "thick") return "5px";
    const match = op.exec(/^(0|(?:\d+(?:\.\d*)?|\.\d+)px)$/, value);
    if (!match || op.float(value) > 8192 || value.length > 256) return undefined;
    if (value === "0") return value;
    let number = op.slice(value, 0, value.length - 2);
    if (number[0] === ".") number = "0" + number;
    if (number[number.length - 1] === ".") number = op.slice(number, 0, number.length - 1);
    return number + "px";
  };
  const line = value => value === "solid" || value === "none" || value === "hidden";
  const expand = (name, value) => {
    value = op.lower(op.trim(value)); const parts = tokens(value), output = op.array();
    const pair = op.mapGet(pairs, name), shorthand = pair && op.own(pair, "border");
    if (shorthand) {
      if (parts.length > 3) fail();
      let w, s, c;
      for (let i = 0; i < parts.length; i++) {
        const token = parts[i], candidate = width(token);
        if (candidate !== undefined) { if (w !== undefined) fail(); w = candidate; }
        else if (line(token)) { if (s !== undefined) fail(); s = token; }
        else if (color(token)) { if (c !== undefined) fail(); c = token; }
        else fail();
      }
      const values = [w ?? "3px", s ?? "none", c ?? "currentcolor"];
      for (let i = 0; i < pair.names.length; i++) op.push(output, { name: pair.names[i], value: values[i % 3] });
      return output;
    }
    const keys = pair ? pair.names : [name], kind = op.exec(/-(width|style|color)$/, name)[1];
    if ((!pair && parts.length !== 1) || parts.length > keys.length) fail();
    for (let i = 0; i < keys.length; i++) {
      let token = parts[i] ?? parts[i === 3 ? 1 : 0] ?? parts[0];
      if (kind === "width") { token = width(token); if (token === undefined) fail(); }
      else if (kind === "style") { if (!line(token)) fail(); }
      else if (!color(token)) fail();
      op.push(output, { name: keys[i], value: token });
    }
    return output;
  };
  const reflect = (name, parts) => {
    if (name === "border") for (let i = 3; i < 12; i++) if (parts[i] !== parts[i % 3]) return undefined;
    return parts[0] + " " + parts[1] + " " + parts[2];
  };
  const project = (css, style) => {
    const used = op.array(), ink = op.array(); let box = false, hasWidth = false, hasColor = false;
    for (let i = 0; i < 4; i++) {
      const w = css[widths[i]], s = css[styles[i]], c = css[colors[i]];
      if (w !== undefined || s !== undefined) hasWidth = true;
      if (c !== undefined) hasColor = true;
      const value = s === "solid" ? w ?? "3px" : "0";
      op.push(used, value); op.push(ink, c ?? "currentcolor");
      if (op.float(value) > 0) box = true;
    }
    const put = (values, suffix) => {
      if (values[0] === values[1] && values[1] === values[2] && values[2] === values[3]) style["border" + suffix] = values[0];
      else for (let i = 0; i < 4; i++) style["border" + titles[i] + suffix] = values[i];
    };
    if (hasWidth) put(used, "Width");
    if (hasColor || hasWidth) put(ink, "Color");
    return box;
  };
  return { names: op.join(names, " "), pairs, expand, reflect, project };
})
