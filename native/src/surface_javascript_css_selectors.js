// Bounded author-sheet matcher; dynamic :hover uses only the original Hover owner's private predicate.
((dom, fail, hovering) => {
  "use strict";
  const op = dom.ops, get = dom.get, Pattern = RegExp;
  // Share mutation identity with the original tree; scan each parent only once per generation instead of rescanning siblings for each rule/node.
  // WeakMap does not retain disconnected nodes; discard the index on generation changes without creating a second tree or lifecycle history.
  let indexedRevision, positions, indexedParents;
  const nth = value => {
    // Bounded ASCII An+B subset; reject unknown of-selectors/nested functions, and keep all integers exactly representable.
    if (!op.exec(/^[\t\n\f\r 0-9a-z+-]+$/i, value)) fail();
    value = op.lower(op.trim(value)); let a = 0, b;
    if (value === "odd" || value === "even") { a = 2; b = value === "odd" ? 1 : 0; }
    else if (op.exec(/^[+-]?\d+$/, value)) b = op.number(value);
    else {
      const terms = op.exec(/^([+-]?\d*)n(?:[\t\n\f\r ]*([+-])[\t\n\f\r ]*(\d+))?$/, value);
      if (!terms) fail();
      a = terms[1] === "" || terms[1] === "+" ? 1 : terms[1] === "-" ? -1 : op.number(terms[1]);
      b = terms[2] ? op.number(terms[2] + terms[3]) : 0;
    }
    if (!op.finite(a) || !op.finite(b) || op.abs(a) > 2147483647 || op.abs(b) > 2147483647) fail();
    return (_, node, ordinal) => {
      const difference = ordinal - b;
      return a === 0 ? difference === 0 : difference % a === 0 && difference / a >= 0;
    };
  };
  const compile = source => {
    source = op.trim(source); if (!source || source.length > 512 || op.index(source, "\\") >= 0) fail();
    const parts = op.array(), combinators = op.array(); let index = 0, ids = 0, classes = 0, tags = 0, positioned = false;
    while (index < source.length) {
      const tests = op.array(); let found = false;
      const tag = op.exec(/^(\*|[a-z][a-z0-9-]*)/i, op.slice(source, index));
      if (tag) { const name = op.lower(tag[0]); op.push(tests, s => name === "*" || s.tag === name); index += tag[0].length; found = true; if (name !== "*") tags++; }
      while (index < source.length && !op.test(/[\s>]/, source[index])) {
        const rest = op.slice(source, index);
        const token = op.exec(/^([.#])([a-z_][a-z0-9_-]*)/i, rest);
        if (token) {
          const value = token[2];
          if (token[1] === "#") { ids++; op.push(tests, s => op.mapGet(s.attrs, "id") === value); }
          else {
            // Tokens contain only admitted ASCII identifiers; the private fixed-boundary regex avoids allocating a per-character words array.
            const pattern = new Pattern("(?:^|[\\t\\n\\f\\r ])" + value + "(?:[\\t\\n\\f\\r ]|$)");
            classes++; op.push(tests, s => op.exec(pattern, op.mapGet(s.attrs, "class") ?? "") !== null);
          }
          index += token[0].length; found = true; continue;
        }
        const attr = op.exec(/^\[([a-z_][a-z0-9_-]*)(?:\s*=\s*(?:"([^"\r\n]*)"|'([^'\r\n]*)'|([a-z0-9_-]+)))?\]/i, rest);
        if (attr) {
          const name = op.lower(attr[1]), value = attr[2] ?? attr[3] ?? attr[4];
          op.push(tests, s => value === undefined ? op.mapHas(s.attrs, name) : op.mapGet(s.attrs, name) === value);
          classes++; index += attr[0].length; found = true; continue;
        }
        const functional = op.exec(/^:nth-child\(([^()]*)\)(?![a-z0-9_-])/i, rest);
        if (functional) {
          op.push(tests, nth(functional[1])); classes++; positioned = true;
          index += functional[0].length; found = true; continue;
        }
        const pseudo = op.exec(/^:(root|disabled|first-child|hover)(?![a-z0-9_-])/i, rest);
        if (!pseudo) fail();
        const name = op.lower(pseudo[1]); classes++;
        if (name === "first-child") positioned = true;
        op.push(tests, (s, node, ordinal) => {
          if (name === "root") return node === dom.root;
          if (name === "disabled") return s.tag === "button" && op.mapHas(s.attrs, "disabled");
          if (name === "hover") return hovering(node);
          return !!s.parent && ordinal === 1;
        });
        index += pseudo[0].length; found = true;
      }
      if (!found || parts.length >= 8 || tests.length > 16) fail();
      op.push(parts, tests);
      const start = index;
      while (index < source.length && op.test(/\s/, source[index])) index++;
      if (index === source.length) break;
      if (source[index] === ">") { op.push(combinators, ">"); index++; while (index < source.length && op.test(/\s/, source[index])) index++; }
      else { if (index === start) fail(); op.push(combinators, " "); }
      if (index === source.length) fail();
    }
    const simple = (node, tests, context) => {
      // Rules in one computed call share target state/indices; predicates are private pure functions with no plugin reentry or cross-frame retention.
      const current = context && context.node === node;
      const state = current ? context.state : get(node); if (state.tag === "#text" || state.tag === "#fragment") return false;
      // Synchronous geometry can reenter from a deep plugin stack; finish indexing before invoking predicates without nesting predicate->position->get.
      let ordinal = 1;
      if (positioned && current && context.ordinal !== undefined) ordinal = context.ordinal;
      else if (positioned && state.parent) {
        const revision = dom.revision();
        if (revision !== indexedRevision) {
          indexedRevision = revision; positions = new op.Weak(); indexedParents = new op.Weak();
        }
        if (!op.weakHas(indexedParents, state.parent)) {
          const children = get(state.parent).children; let count = 0;
          for (let i = 0; i < children.length; i++) {
            const tag = get(children[i]).tag;
            if (tag !== "#text" && tag !== "#fragment") op.weakSet(positions, children[i], ++count);
          }
          op.weakSet(indexedParents, state.parent, true);
        }
        ordinal = op.weakGet(positions, node);
        if (current) context.ordinal = ordinal;
      }
      for (let i = 0; i < tests.length; i++) if (!tests[i](state, node, ordinal)) return false;
      return true;
    };
    return { specificity: [ids, classes, tags], matches(node, context) {
      // Single-part selectors have no ancestor constraints and match directly; do not allocate a backtracking work array for each rule/node.
      if (parts.length === 1) return simple(node, parts[0], context);
      // Ancestor depth is 32; deduplicate reachable states to avoid combinatorial backtracking/heap growth from repeated descendants.
      const ancestors = op.array();
      for (let current = node; current; current = get(current).parent) op.push(ancestors, current);
      let reachable = op.array(); reachable[0] = true;
      for (let part = parts.length - 1; part >= 0; part--) {
        const next = op.array();
        for (let i = 0; i < ancestors.length; i++) {
          if (!reachable[i] || !simple(ancestors[i], parts[part], context)) continue;
          if (part === 0) return true;
          for (let j = i + 1; j < ancestors.length; j++) {
            next[j] = true; if (combinators[part - 1] === ">") break;
          }
        }
        reachable = next;
      }
      return false;
    } };
  };
  const list = source => {
    // Commas in attribute values are not selector separators; nth-child rejects commas, of-selectors and nested functions.
    const result = op.array(); let start = 0, bracket = false, quote = "";
    for (let i = 0; i <= source.length; i++) {
      const c = source[i];
      if (quote) { if (c === quote) quote = ""; }
      else if (c === '"' || c === "'") quote = c;
      else if (c === "[") { if (bracket) fail(); bracket = true; }
      else if (c === "]") { if (!bracket) fail(); bracket = false; }
      else if ((c === "," && !bracket) || i === source.length) {
        if (result.length >= 16) fail(); op.push(result, compile(op.slice(source, start, i))); start = i + 1;
      }
    }
    if (quote || bracket) fail(); return result;
  };
  return { list, context: node => ({ node, state: get(node), ordinal: undefined }) };
})
