// Constructed sheets 与 author cascade；无 URL/@import/异步加载，采用/替换均原子提交。
((dom, fail, values, selectors, declarations) => {
  "use strict";
  const op = dom.ops, sheets = new op.Weak(); let created = 0, adopted = op.array();
  const rulesOf = sheet => op.weakGet(sheets, sheet) ?? fail();
  // 与inline声明相同，用户转换必须在进入私有parser之前完成，允许有界重入。
  const parse = source => {
    source = declarations.strip(source); const rules = op.array(); let offset = 0;
    while (offset < source.length) {
      if (!op.trim(op.slice(source, offset))) break;
      const open = op.index(source, "{", offset), close = op.index(source, "}", open + 1);
      if (open < 0 || close < 0 || rules.length >= 128) fail();
      const selector = op.slice(source, offset, open), body = op.slice(source, open + 1, close);
      if (op.index(selector, "@") >= 0 || op.test(/[{}]/, body)) fail();
      const matches = selectors.list(selector), store = declarations.parse(body), entries = op.array();
      // sheet规则不可逐项修改；解析后只持这一份有序声明，不为每次node匹配重建Map iterator。
      const iterator = op.mapEntries(store); let next;
      while (!(next = op.mapNext(iterator)).done) op.push(entries, next.value);
      op.push(rules, { selectors: matches, declarations: entries }); offset = close + 1;
    }
    return rules;
  };
  const budget = (next, replaced, rules) => {
    let count = 0;
    for (let i = 0; i < next.length; i++) { count += (next[i] === replaced ? rules : rulesOf(next[i])).length; if (count > 256) fail(); }
  };
  class CSSStyleSheet {
    constructor(...options) { if (options.length || ++created > 16) fail(); op.weakSet(sheets, this, op.array()); op.freeze(this); }
    replaceSync(source) { rulesOf(this); const rules = parse(op.string(source)); budget(adopted, this, rules); dom.changed(); op.weakSet(sheets, this, rules); }
  }
  const adopt = next => {
    if (!op.isArray(next) || next.length > 16) fail();
    const copy = op.array(), length = next.length;
    for (let i = 0; i < length; i++) { const sheet = next[i]; rulesOf(sheet); op.push(copy, sheet); }
    budget(copy); dom.changed(); adopted = copy;
  };
  const adoptedCopy = () => { const copy = []; for (let i = 0; i < adopted.length; i++) copy[i] = adopted[i]; return copy; };
  // 编译器始终生成[id,class,tag]三项元组，直接逐项比较，不丢失任何权重。
  const greater = (a, b) => a[0] !== b[0] ? a[0] > b[0] :
    a[1] !== b[1] ? a[1] > b[1] : a[2] >= b[2];
  const computed = (node, parent) => {
    // key仅来自已准入的canonical CSS声明；空原型record不读取插件原型，生命周期只限本次计算。
    const winners = op.record(), context = selectors.context(node);
    // 同步 getter 可从插件调用栈进入；捕获的 iterator 避免 forEach 再嵌套一次 JS/native 回调。
    const apply = (store, specificity, inline) => {
      // inline仍读取原可变Map；author sheet使用解析时的私有有序声明，优先级/源序不变。
      const iterator = inline ? op.mapEntries(store) : null; let index = 0, next;
      while (inline ? !(next = op.mapNext(iterator)).done : index < store.length) {
        const entry = inline ? next.value : store[index++], name = entry[0], item = entry[1];
        // 输入按源序遍历，相同优先级的后来声明自然覆盖；不为每次匹配分配六项rank数组。
        // specificity仍逐项比较，不压成会碰撞的数值；important优先于独立的inline层级。
        const important = item.priority === "important", previous = winners[name];
        const wins = !previous || (important !== previous.important ? important :
          inline !== previous.inline ? inline > previous.inline : greater(specificity, previous.specificity));
        if (wins && previous) {
          // winner只活在本次computed内；更新已有自有字段，不为相同属性的后来声明反复分配。
          previous.important = important; previous.inline = inline;
          previous.specificity = specificity; previous.value = item.value;
        } else if (wins) winners[name] = { important, inline, specificity, value: item.value };
      }
    };
    for (let i = 0; i < adopted.length; i++) {
      const rules = rulesOf(adopted[i]);
      for (let j = 0; j < rules.length; j++) {
        const rule = rules[j]; let specificity;
        for (let k = 0; k < rule.selectors.length; k++) {
          const selector = rule.selectors[k];
          if (selector.matches(node, context) && (!specificity || greater(selector.specificity, specificity))) specificity = selector.specificity;
        }
        if (specificity) apply(rule.declarations, specificity, 0);
      }
    }
    apply(declarations.get(node), [0, 0, 0], 1);
    const result = op.record();
    for (const name in winners) result[name] = winners[name].value;
    for (let i = 0; i < values.inherited.length; i++) {
      const name = values.inherited[i];
      if (!op.own(result, name) && parent && op.own(parent, name)) result[name] = parent[name];
    }
    if (result.color === "currentcolor") { if (parent?.color) result.color = parent.color; else delete result.color; }
    return result;
  };
  return { CSSStyleSheet, adopt, adoptedCopy, computed };
})
