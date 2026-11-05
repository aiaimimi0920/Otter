// 有界轨道语法规范化；非递归扫描/展开，避免在原 256 KiB 栈上叠加 parser 调用。
((dom, fail) => {
  "use strict";
  const op = dom.ops;
  const tokenize = (source, maximumDepth) => {
    const result = op.array(); let start = 0, depth = 0;
    for (let i = 0; i <= source.length; i++) {
      const c = source[i];
      if (c === "(") { if (++depth > maximumDepth) fail(); }
      else if (c === ")") { if (--depth < 0) fail(); }
      else if (i === source.length || (!depth && (c === " " || c === "\t" || c === "\n" || c === "\f" || c === "\r"))) {
        if (depth) fail();
        const token = op.trim(op.slice(source, start, i)); start = i + 1;
        if (token) op.push(result, token);
        if (result.length > 64) fail();
      }
    }
    if (!result.length) fail(); return result;
  };
  return (source, template) => {
    if (template && source === "none") return source;
    const tokens = tokenize(source, template ? 2 : 1), output = op.array();
    for (let i = 0; i < tokens.length; i++) {
      const repeat = op.exec(/^repeat\(\s*(\d+)\s*,([\s\S]+)\)$/, tokens[i]);
      const count = repeat ? op.number(repeat[1]) : 1;
      const inner = repeat ? tokenize(op.trim(repeat[2]), 1) : op.array();
      if (!repeat) op.push(inner, tokens[i]);
      if ((repeat && !template) || !count || count > 64 || count * inner.length + output.length > 64) fail();
      const normalized = op.array();
      for (let j = 0; j < inner.length; j++) {
        const minmax = op.exec(/^minmax\(([^(),]+),([^(),]+)\)$/, inner[j]), bounds = op.array();
        if (minmax) { op.push(bounds, op.trim(minmax[1])); op.push(bounds, op.trim(minmax[2])); }
        else op.push(bounds, inner[j]);
        for (let k = 0; k < bounds.length; k++) {
          let value = bounds[k];
          if (value === "auto" || value === "0") continue;
          if (value[0] === ".") value = "0" + value;
          if (value.length > 256) fail();
          const match = op.exec(/^(\d+(?:\.\d+)?)(px|%|fr)$/, value);
          if (!match) fail();
          const number = op.float(match[1]), unit = match[2];
          if (!op.finite(number) || number > (unit === "px" ? 8192 : 1000) ||
              (unit === "fr" && ((minmax && k === 0) || op.fround(number) <= 0))) fail();
          bounds[k] = value;
        }
        op.push(normalized, minmax ? "minmax(" + bounds[0] + "," + bounds[1] + ")" : bounds[0]);
      }
      for (let n = 0; n < count; n++) for (let j = 0; j < normalized.length; j++) op.push(output, normalized[j]);
    }
    if (!template && output.length !== 1) fail();
    const result = op.join(output, " "); if (result.length > 4096) fail(); return result;
  };
})
