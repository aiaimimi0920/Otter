// Convert computed declarations to real Scene keys; preserve Otter's default document model without simulating unimplemented browser boxes or inline flow.
((dom, fail, values, sheets, spatial, canvas, form) => {
  "use strict";
  const op = dom.ops;
  const project = root => {
    const computed = new op.Weak(), pending = op.array(); op.push(pending, { node: root, parent: null });
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      const state = dom.get(item.node); let css;
      if (state.tag === "#text") {
        css = op.record();
        for (let i = 0; i < values.inherited.length; i++) { const name = values.inherited[i]; if (item.parent && op.own(item.parent, name)) css[name] = item.parent[name]; }
      } else css = sheets.computed(item.node, item.parent);
      op.weakSet(computed, item.node, css);
      for (let i = state.children.length - 1; i >= 0; i--) op.push(pending, { node: state.children[i], parent: css });
    }
    // Only prepare uses native projection; merge this layer to preserve the original stack budget during public value-conversion reentry.
    const prepare = node => {
      const state = dom.get(node), css = op.weakGet(computed, node);
      const parent = state.parent && op.weakGet(computed, state.parent);
      const container = op.setHas(dom.containers, state.tag);
      spatial.order(node, computed);
      spatial.validate(node, css, parent);
      const lineHeight = values.lineHeight(css);
      const layout = op.record(), style = op.record(); let box = false, border = false;
      const grid = css.display === "grid", flex = css.display === "flex", hidden = css.display === "none";
      const gridContainer = ["grid-template-columns", "grid-template-rows", "grid-auto-columns", "grid-auto-rows", "grid-auto-flow", "justify-items", "align-content"];
      const gridItem = ["grid-column-start", "grid-column-end", "grid-row-start", "grid-row-end", "justify-self", "align-self"];
      const shared = ["row-gap", "column-gap", "justify-content", "align-items"];
      const any = names => { for (let i = 0; i < names.length; i++) if (op.own(css, names[i])) return true; return false; };
      if ((grid || flex || any(gridContainer) || any(shared) || css["flex-direction"]) && !container) fail();
      if (any(gridContainer) && !grid && !hidden) fail();
      if (any(shared) && !grid && !flex && !hidden) fail();
      if (css["flex-direction"] && !flex && !hidden) fail();
      // Hidden items may retain known declarations; recheck context when visible again rather than storing and ignoring them in normal flow.
      if (any(gridItem) && !hidden && parent?.display !== "grid" && parent?.display !== "none") fail();
      if (op.own(css, "flex-grow") && parent?.display !== "flex" && parent?.display !== "none") fail();
      if (flex && (css["justify-content"] === "stretch" || css["align-items"] === "baseline")) fail();
      const remap = name => name === "background-color" ? "background" : name === "flex-grow" ? "grow" :
        name === "justify-content" ? "justify" : name === "align-items" ? "align" : undefined;
      // css is a private null-prototype record containing only admitted declarations/inherited values; do not scan the entire definition table.
      // Do not call public keys/iterators or change context validation, alias precedence or native value conversion.
      for (const name in css) {
        const definition = op.mapGet(values.definitions, name);
        if (definition.kind === "border") { border = true; continue; }
        if (op.indexOf(["display", "flex-direction", "box-sizing", "border-style", "row-gap", "column-gap"], name) >= 0 ||
            op.test(/^grid-(?:row|column)-(?:start|end)$/, name)) continue;
        if (name === "background" && op.own(css, "background-color")) continue;
        const key = remap(name) ?? definition.key;
        const target = definition.target === "layout" ? layout : style;
        const numeric = name === "font-weight" || op.indexOf(["opacity", "grow"], definition.kind) >= 0;
        let value = name === "line-height" ? lineHeight : css[name];
        // Scene self normal is equivalent to auto; CSS normal for non-replaced Grid items means stretch.
        if (parent?.display === "grid" && (name === "align-self" || name === "justify-self") && value === "normal") value = "stretch";
        if (flex && (name === "justify-content" || name === "align-items")) {
          if (value === "start" || value === "end") value = "flex-" + value;
          if (value === "normal") value = name === "align-items" ? "stretch" : "flex-start";
        }
        target[key] = numeric ? op.number(value) : value;
        if (definition.kind === "size" || op.index(name, "padding-") === 0 || name === "border-width") box = true;
      }
      if (border && values.borders.project(css, style)) box = true;
      // Native dimensions are border-box; do not silently treat CSS's default content-box as equivalent.
      if (box && css["box-sizing"] !== "border-box") fail();
      const type = container ? grid ? "grid" : flex ? css["flex-direction"] ?? "row" : "column" : state.tag === "canvas" ? "image" : state.tag === "input" ? "input" : state.tag === "button" ? "button" : "text";
      if (grid) {
        layout.rowGap = css["row-gap"] ?? 0; layout.columnGap = css["column-gap"] ?? 0;
      } else if (container) layout.gap = flex ? css[css["flex-direction"] === "column" ? "row-gap" : "column-gap"] ?? 0 : 6;
      for (let i = 0; i < 2; i++) {
        const axis = i ? "row" : "column", start = css["grid-" + axis + "-start"], end = css["grid-" + axis + "-end"];
        if (start === undefined && end === undefined) continue;
        if (op.index(start ?? "", "span ") === 0 && op.index(end ?? "", "span ") === 0) fail();
        layout[i ? "gridRow" : "gridColumn"] = (start ?? "auto") + " / " + (end ?? "auto");
      }
      // Grid/flex items with auto width must not inherit the native normal container's full-width default.
      if ((css.position === "absolute" || parent?.display === "grid" || parent?.display === "flex") && !op.own(css, "width")) layout.width = "auto";
      const props = state.tag === "input" ? form.props(node) : op.record();
      if (css.display === "none") props.visible = false;
      if (state.tag === "canvas") {
        props._canvas = canvas.dimensions(node);
        if (props._canvas[0] === 0 || props._canvas[1] === 0) {
          if (!op.own(css,"width")) layout.width = props._canvas[0];
          if (!op.own(css,"height")) layout.height = props._canvas[1];
        }
      } else if (!container && state.tag !== "input") props[state.tag === "button" ? "label" : "text"] = dom.text(node);
      if (state.tag === "button") props.disabled = op.mapHas(state.attrs, "disabled");
      const output = container ? { id: state.key, type, props, layout, style, children: op.array() } : { id: state.key, type, props, layout, style };
      if (container) op.push(pending, { state, output });
      else if (state.tag !== "canvas") {
        // Existing text/button leaves flatten inline descendants with no separate run styles or geometry; differing styles must be rejected.
        const descendants = op.array();
        for (let i = 0; i < state.children.length; i++) op.push(descendants, state.children[i]);
        while (descendants.length) {
          const child = descendants[descendants.length - 1]; descendants.length--;
          const s = dom.get(child), childCss = op.weakGet(computed, child);
          if (s.tag !== "#text") {
            const iterator = op.mapEntries(values.definitions); let next;
            while (!(next = op.mapNext(iterator)).done) {
              const name = next.value[0];
              if (name !== "box-sizing" && op.own(childCss, name) &&
                  (op.indexOf(values.inherited, name) < 0 || childCss[name] !== css[name])) fail();
            }
          }
          for (let i = 0; i < s.children.length; i++) op.push(descendants, s.children[i]);
        }
      }
      return output;
    };
    const output = prepare(root);
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      for (let i = 0; i < item.state.children.length; i++) {
        const child = item.state.children[i], state = dom.get(child);
        if (state.tag !== "#text" || op.test(/\S/, state.data)) op.push(item.output.children, prepare(child));
      }
    }
    return output;
  };
  return project;
})
