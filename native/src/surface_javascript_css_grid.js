// 受支持 Grid 声明展开/端点/对齐；只输出 longhand，负线/放置仍由原生 owner 求值。
((dom, fail, tracks) => {
  "use strict";
  const op = dom.ops, pairs = new op.MapType(), alignments = new op.MapType();
  const pair = (name, first, second, separator) => op.mapSet(pairs, name, { names: op.words(first + " " + second), separator });
  pair("gap", "row-gap", "column-gap", " ");
  pair("grid-row", "grid-row-start", "grid-row-end", " / ");
  pair("grid-column", "grid-column-start", "grid-column-end", " / ");
  pair("place-items", "align-items", "justify-items", " ");
  pair("place-self", "align-self", "justify-self", " ");
  pair("place-content", "align-content", "justify-content", " ");
  const item = "normal stretch start end flex-start flex-end center", content = item + " space-between space-around space-evenly";
  for (const [name, choices] of [
    ["align-items", item + " baseline"], ["justify-items", item],
    ["align-self", item + " baseline auto"], ["justify-self", item + " auto"],
    ["align-content", content], ["justify-content", content],
  ]) op.mapSet(alignments, name, op.words(choices));
  const names = "gap row-gap column-gap grid-template-columns grid-template-rows grid-auto-columns grid-auto-rows grid-auto-flow grid-column grid-row grid-column-start grid-column-end grid-row-start grid-row-end justify-items align-items justify-self align-self justify-content align-content place-items place-self place-content";
  const expand = (name, value) => {
    const pair = op.mapGet(pairs, name), output = op.array();
    if (!pair) op.push(output, { name, value });
    else {
      const lines = pair.separator === " / ", items = lines ? op.split(value, "/") : op.words(value);
      if (!items.length || items.length > 2) fail();
      op.push(output, { name: pair.names[0], value: op.trim(items[0]) });
      op.push(output, { name: pair.names[1], value: items.length === 2 ? op.trim(items[1]) : lines ? "auto" : output[0].value });
    }
    // 不通过 validate→expand→single→track 的层叠递归；保留原 heap/stack/时间预算。
    for (let i = 0; i < output.length; i++) {
      const item = output[i], name = item.name; let value = item.value;
      if (op.index(name, "grid-template-") === 0 || name === "grid-auto-columns" || name === "grid-auto-rows") {
        value = tracks(value, op.index(name, "grid-template-") === 0);
      } else if (name === "grid-auto-flow") { if (value !== "row") fail(); }
      else if (op.index(name, "grid-") === 0 && value !== "auto") {
        const span = op.exec(/^(?:span\s+([+]?\d+)|([+]?\d+)\s+span)$/, value);
        if (!span && !op.exec(/^[+-]?\d+$/, value)) fail();
        const number = op.number(span ? span[1] ?? span[2] : value);
        if (!number || op.abs(number) > 2049) fail();
        value = (span ? "span " : "") + op.string(number);
      } else if (name === "row-gap" || name === "column-gap") {
        if (!op.exec(/^(?:0|(?:\d+(?:\.\d+)?|\.\d+)px)$/, value) || op.float(value) > 8192) fail();
        if (value[0] === ".") value = "0" + value;
        if (value.length > 256) fail();
      } else if (op.mapHas(alignments, name) && op.indexOf(op.mapGet(alignments, name), value) < 0) fail();
      item.value = value;
    }
    if (pair?.separator === " / " && op.index(output[0].value, "span ") === 0 && op.index(output[1].value, "span ") === 0) fail();
    return output;
  };
  return { names, pairs, expand };
})
