// 计算声明转换为真实 Scene 键；保留 Otter 文档默认模型，不模拟未实现的浏览器盒/inline flow。
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
    // native 投影只由 prepare 使用；合并这一层以保留公开值转换重入时的原栈预算。
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
      // 隐藏项可保留已知声明；恢复可见时重新检查上下文，不能在普通 flow 中保存后忽略。
      if (any(gridItem) && !hidden && parent?.display !== "grid" && parent?.display !== "none") fail();
      if (op.own(css, "flex-grow") && parent?.display !== "flex" && parent?.display !== "none") fail();
      if (flex && (css["justify-content"] === "stretch" || css["align-items"] === "baseline")) fail();
      const remap = name => name === "background-color" ? "background" : name === "flex-grow" ? "grow" :
        name === "justify-content" ? "justify" : name === "align-items" ? "align" : undefined;
      // css 是私有 null-prototype 记录，仅含已准入的声明/继承值；不扫描整张定义表。
      // 不调用公开 keys/iterator，也不改变上下文校验、别名优先级或 native 值转换。
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
        // Scene 的 self normal 等同 auto；CSS 非替换 Grid item 的 normal 则是 stretch。
        if (parent?.display === "grid" && (name === "align-self" || name === "justify-self") && value === "normal") value = "stretch";
        if (flex && (name === "justify-content" || name === "align-items")) {
          if (value === "start" || value === "end") value = "flex-" + value;
          if (value === "normal") value = name === "align-items" ? "stretch" : "flex-start";
        }
        target[key] = numeric ? op.number(value) : value;
        if (definition.kind === "size" || op.index(name, "padding-") === 0 || name === "border-width") box = true;
      }
      if (border && values.borders.project(css, style)) box = true;
      // Native 尺寸是 border-box；不能把 CSS 缺省 content-box 静默当作相同语义。
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
      // Grid/flex item 的 auto width 不能沿用 native 普通容器的 full-width 缺省。
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
        // 原文字/按钮叶压平 inline 后代，没有独立 run 样式或几何；不同样式必须拒绝。
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
