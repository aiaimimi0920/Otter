// Constructed sheets and author cascade; no URLs, @import or asynchronous loading, with atomic adoption/replacement.
((dom, fail, values, selectors, declarations) => {
  "use strict";
  const op = dom.ops, sheets = new op.Weak(); let created = 0, adopted = op.array();
  const rulesOf = sheet => op.weakGet(sheets, sheet) ?? fail();
  // As with inline declarations, finish user conversion before entering the private parser, allowing bounded reentry.
  const parse = source => {
    source = declarations.strip(source); const rules = op.array(); let offset = 0;
    while (offset < source.length) {
      if (!op.trim(op.slice(source, offset))) break;
      const open = op.index(source, "{", offset), close = op.index(source, "}", open + 1);
      if (open < 0 || close < 0 || rules.length >= 128) fail();
      const selector = op.slice(source, offset, open), body = op.slice(source, open + 1, close);
      if (op.index(selector, "@") >= 0 || op.test(/[{}]/, body)) fail();
      const matches = selectors.list(selector), store = declarations.parse(body), entries = op.array();
      // Sheet rules cannot be modified individually; retain this single ordered declaration list after parsing instead of rebuilding Map iterators for each node match.
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
  // The compiler always produces three-element [id,class,tag] tuples; compare each component directly without losing any weight.
  const greater = (a, b) => a[0] !== b[0] ? a[0] > b[0] :
    a[1] !== b[1] ? a[1] > b[1] : a[2] >= b[2];
  const computed = (node, parent) => {
    // Keys come only from admitted canonical CSS declarations; null-prototype records do not read plugin prototypes and live only for this computation.
    const winners = op.record(), context = selectors.context(node);
    // Synchronous getters can enter from the plugin call stack; the captured iterator avoids an extra JS/native callback layer from forEach.
    const apply = (store, specificity, inline) => {
      // Inline still reads the original mutable Map; author sheets use private ordered declarations created during parsing, preserving precedence/source order.
      const iterator = inline ? op.mapEntries(store) : null; let index = 0, next;
      while (inline ? !(next = op.mapNext(iterator)).done : index < store.length) {
        const entry = inline ? next.value : store[index++], name = entry[0], item = entry[1];
        // Traverse input in source order so later declarations of equal precedence naturally override earlier ones; avoid allocating six-element rank arrays per match.
        // Compare specificity component by component rather than collapsing it into a collision-prone number; important precedes the separate inline tier.
        const important = item.priority === "important", previous = winners[name];
        const wins = !previous || (important !== previous.important ? important :
          inline !== previous.inline ? inline > previous.inline : greater(specificity, previous.specificity));
        if (wins && previous) {
          // Winners live only within this computed call; update existing own fields instead of reallocating for later declarations of the same property.
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
