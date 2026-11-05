// 私有桥只负责生命周期/emit/投影；文档状态由独立工厂封装，不解释插件业务代码。
((makeTree, attributes, selectors, markup, json, cssGridTracks, cssGrid, cssSpatial, cssValues, cssSelectors, cssDeclarations, cssSheets, cssProjection, makeGeometry, makeAnimation, makeTimers, makeResize, makeCanvas, makeCanvasContext, makeCanvasPath, makeFonts, makeEvents, makeForm, makeFocus, makeHover, makeBorders, makeServices, makeResult, makeImages) => {
  "use strict";
  const clone = json.clone, freeze = Object.freeze;
  let definition, cleanup, snapshot = null, resources = {};
  let mounted = false, suspended = false, disposed = false, emitted, defaultAllowed = null, utf8Length;
  const safe = value => typeof value === "string" && /^[A-Za-z0-9:_-]{1,256}$/.test(value);
  const fail = () => { throw new Error("Unsupported or invalid native DOM operation"); };
  const dom = makeTree(fail, makeEvents), { nodes, root, ops: op } = dom;
  emitted = op.array();
  const form = makeForm(dom, fail);
  const focus = makeFocus(dom, fail);
  attributes(dom, fail, form); const query = selectors(dom, fail).query; markup(dom, fail);
  const spatial = cssSpatial(dom, fail);
  const values = cssValues(dom, fail, cssGrid(dom, fail, cssGridTracks(dom, fail)), spatial, makeFonts(dom, fail), makeBorders), declarations = cssDeclarations(dom, fail, values);
  // 工厂构造不求值样式；插件运行前完成同一 Hover owner 的双向接线。
  let hover;
  const sheets = cssSheets(dom, fail, values, cssSelectors(dom, fail, node => hover.matches(node)), declarations);
  hover = makeHover(dom, fail, sheets);
  const images = makeImages(op,json,fail);
  const canvas = makeCanvas(dom, fail, values, json, makeCanvasContext, makeCanvasPath, images);
  const services = makeServices(json, fail, images, canvas);
  const project = cssProjection(dom, fail, values, sheets, spatial, canvas, form);
  const geometry = makeGeometry(dom, fail, project, json);
  const animation = makeAnimation(dom, fail);
  const timers = makeTimers(dom, fail, animation.now);
  const resize = makeResize(dom, fail, geometry);
  Object.defineProperty(globalThis, "CSSStyleSheet", { value: sheets.CSSStyleSheet });
  root.id = "surface-root";
  for (const [name, value] of Object.entries(dom.classes)) Object.defineProperty(globalThis, name, { value });
  const document = Object.freeze({
    createElement: dom.createElement,
    createTextNode: text => new dom.Text(text),
    createDocumentFragment: () => new dom.DocumentFragment(),
    getElementById: id => {
      const values = query(root, "*", true), expected = op.string(id);
      for (let i = 0; i < values.length; i++) if (values[i].id === expected) return values[i];
      return null;
    },
    querySelectorAll: selector => query(root, selector, true),
    querySelector: selector => query(root, selector, true)[0] ?? null,
    get adoptedStyleSheets() { return sheets.adoptedCopy(); },
    set adoptedStyleSheets(value) { sheets.adopt(value); },
  });
  const result = makeResult(json, fail, id => {
    const node = document.getElementById(id);
    if (!node || dom.get(node).tag !== "canvas") fail();
    return dom.get(node).key;
  });
  const declaration = (node, id) => {
    const pending = op.array(); if (node) op.push(pending, node);
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      if (item.id === id) return item;
      const children = item.children;
      for (let i = children ? children.length - 1 : -1; i >= 0; i--) op.push(pending, children[i]);
    }
  };
  const emit = request => {
    if (!mounted || suspended || disposed || emitted.length >= 32) return false;
    if (!request || !safe(request.nodeId) || !safe(request.event) || !safe(request.action) ||
        op.indexOf(["discrete", "continuous", "commit"], request.class) < 0) return false;
    if (declaration(snapshot?.scene, request.nodeId)?.events?.[request.event] !== request.action) return false;
    try {
      const event = { nodeId: request.nodeId, event: request.event, action: request.action,
        class: request.class, payload: request.payload ?? null };
      const encoded = json.payload(event.payload);
      // UTF-16 长度是 UTF-8 字节数下界；ASCII 快路避免逐字符迭代器/临时字符串。
      const limit = 60 * 1024;
      let bytes = encoded.length;
      if (bytes > limit) return false;
      // 避免 QuickJS 对补充平面字符串逐次 charCodeAt 转换；宿主只计算同一 JSON 的 UTF-8 长度。
      // UTF-16 上界已检查，最多转换 180 KiB，不暴露文件/网络或其他宿主能力。
      if (/[^\x00-\x7F]/.test(encoded)) bytes = utf8Length(encoded);
      if (bytes > limit) return false;
      // 首个子集保留 4 KiB 给宿主身份/envelope；父侧仍执行完整 64 KiB wire 门。
      event.payload = json.decode(encoded); op.push(emitted, event); return true;
    } catch { return false; }
  };
  Object.defineProperty(globalThis, "document", { value: document });
  Object.defineProperty(globalThis, "NeuroSurface", { value: Object.freeze({
    define(value) {
      if (definition || !value || typeof value.mount !== "function") fail();
      definition = Object.freeze({ mount: value.mount, update: value.update, suspend: value.suspend,
        resume: value.resume, dispose: value.dispose, onLiveTrigger: value.onLiveTrigger });
    },
    emit, service: services.call, serviceImages: services.callImages, serviceCanvases: services.callCanvases, serviceImage: services.callImage,
    serviceWithConfirmation: services.callWithConfirmation,
    commitResult: result.commit, image: images.image, snapshot: () => clone(snapshot), resource: id => Object.hasOwn(resources, id) ? resources[id] : undefined,
  }) });
  // apply 已拥有 Promise 边界；直接调生命周期，避免同步 getter 再套一层 async 栈。
  const apply = async json => {
    const command = protocol.decode(json); emitted = op.array(); defaultAllowed = null;
    const callback = command.type === "liveTrigger";
    services.begin(!disposed && command.type !== "dispose" && command.type !== "suspend" && (!suspended || command.type === "resume" || callback));
    result.begin(!disposed && command.type !== "dispose" && command.type !== "suspend" && (!suspended || command.type === "resume" || callback));
    focus.begin(!callback && !disposed && command.type !== "dispose" && command.type !== "suspend" && (!suspended || command.type === "resume"));
    if (disposed) fail();
    if (command.type === "mount") {
      if (mounted || !definition) fail();
      snapshot = command.snapshot; resources = freeze(command.resources); mounted = true;
      cleanup = await definition.mount({ root, snapshot: clone(snapshot), resources, emit });
    } else if (command.type === "update") {
      snapshot = command.snapshot; resources = freeze(command.resources);
      if (typeof definition.update === "function") await definition.update({ snapshot: clone(snapshot), resources });
    } else if (command.type === "click") {
      const target = op.mapGet(nodes, command.target);
      if (suspended || !target || op.mapHas(dom.get(target).attrs, "disabled")) fail();
      let current = target; const path = op.array();
      while (current) { op.push(path, current); current = dom.get(current).parent; }
      if (op.indexOf(path, root) < 0) fail();
      await dom.events.click(target);
    } else if (command.type === "localInput") {
      if (suspended) fail();
      const input = command.input.event;
      if (command.input.kind === "keyboard") defaultAllowed = await dom.events.keyboard(form.target(input.target), input);
      else if (command.input.kind === "hover") await hover.accept(input);
      else if (command.input.kind === "pointer") {
        const target = op.mapGet(nodes, input.target);
        const connected = (visible = false) => { for (let current = target; current; current = dom.get(current).parent) {
          if (visible && sheets.computed(current).display === "none") return false;
          if (current === root) return true;
        } return false; };
        if (!target || dom.get(target).tag !== "canvas" || (input.phase !== "pointercancel" && !connected())) fail();
        await dom.events.pointer(target, input);
        // Pointer 没有原生默认动作；click 保持原阈值，在 up/microtask 后重验连接身份。
        if (input.click && connected(true) && !op.mapHas(dom.get(target).attrs, "disabled")) await dom.events.click(target);
      }
      else if (command.input.kind === "text" && (input.phase === "input" || input.phase === "change"))
        await dom.events.native(form.accept(input), input.phase, false, input.phase === "input");
      else fail();
    } else if (callback) {
      if (!mounted || typeof definition.onLiveTrigger !== "function") fail();
      // 自动回调不制造点击/gesture，不恢复暂停的timer或给宿主焦点请求。
      await definition.onLiveTrigger(clone(command.event));
    } else if (command.type === "animationFrame") {
      if (suspended) fail(); timers.begin(); animation.begin();
    } else if (command.type === "timerTick") {
      if (suspended) fail(); timers.begin();
    } else if (command.type === "suspend") {
      suspended = true; timers.suspend(); hover.clear(); if (typeof definition.suspend === "function") await definition.suspend(undefined);
    } else if (command.type === "resume") {
      if (typeof definition.resume === "function") await definition.resume(undefined); timers.resume(); suspended = false;
    } else if (command.type === "dispose") {
      disposed = true;
      try { if (typeof cleanup === "function") await cleanup(); }
      finally { if (typeof definition.dispose === "function") await definition.dispose(undefined); }
      root.replaceChildren(); op.mapClear(nodes);
      geometry.dispose();
      animation.dispose();
      timers.dispose();
      resize.dispose();
      canvas.dispose();
      images.dispose();
      hover.clear();
      dom.events.dispose();
    } else fail();
  };
  // apply 的 Promise 和所有可运行 microtask 完成后，Rust 才 collect，避免丢晚到 emit。
  const protocol = json;
  return { apply, endServices() { services.end(); result.end(); },
    bindUtf8Length(value) { if (utf8Length || typeof value !== "function") fail(); utf8Length = value; },
    bindImages: images.bind,
    nextService: services.next, settleService: services.settle,
    bindGeometry: geometry.bind, beginGeometry: geometry.begin,
    bindAnimationClock(value) { animation.bind(value); dom.events.bindClock(value); }, animationStep: animation.step,
    timerStep: timers.step,
    beginResize: resize.begin, resizeStep: () => !suspended && !disposed && resize.step(), bindCanvas: canvas.bind, beginCanvas: canvas.begin,
    collect: () => protocol.encode({ result: result.collect(), scene: project(root), focus: focus.collect(), defaultAllowed, canvases: disposed ? [] : canvas.collect(), events: emitted, disposed, layoutRead: geometry.used(), animationPending: !suspended && animation.pending(), timerDelayMs: timers.pending(), resizeObserved: !suspended && resize.observing() }) };
})
