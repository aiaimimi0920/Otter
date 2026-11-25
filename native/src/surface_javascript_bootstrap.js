// The private bridge handles only lifecycle/emit/projection; a separate factory encapsulates document state without interpreting plugin business code.
((makeTree, attributes, selectors, markup, json, cssGridTracks, cssGrid, cssSpatial, cssValues, cssSelectors, cssDeclarations, cssSheets, cssProjection, makeGeometry, makeAnimation, makeTimers, makeResize, makeCanvas, makeCanvasContext, makeCanvasPath, makeFonts, makeEvents, makeForm, makeFocus, makeHover, makeBorders, makeServices, makeResult, makeImages) => {
  "use strict";
  const clone = json.clone, freeze = Object.freeze, ErrorType = Error;
  let definition, cleanup, snapshot = null, resources = {};
  let mounted = false, suspended = false, disposed = false, emitted, defaultAllowed = null, utf8Length;
  const safe = value => typeof value === "string" && /^[A-Za-z0-9:_-]{1,256}$/.test(value);
  const fail = () => { throw new ErrorType("Unsupported or invalid native DOM operation"); };
  const dom = makeTree(fail, makeEvents), { nodes, root, ops: op } = dom;
  emitted = op.array();
  const form = makeForm(dom, fail);
  const focus = makeFocus(dom, fail);
  attributes(dom, fail, form); const { query, first } = selectors(dom, fail);
  const spatial = cssSpatial(dom, fail);
  const values = cssValues(dom, fail, cssGrid(dom, fail, cssGridTracks(dom, fail)), spatial, makeFonts(dom, fail), makeBorders), declarations = cssDeclarations(dom, fail, values);
  // Factory construction does not evaluate styles; wire the same Hover owner in both directions before running the plugin.
  let hover;
  const sheets = cssSheets(dom, fail, values, cssSelectors(dom, fail, node => hover.matches(node)), declarations);
  hover = makeHover(dom, fail, sheets);
  const images = makeImages(op,json,fail);
  const canvas = makeCanvas(dom, fail, values, json, makeCanvasContext, makeCanvasPath, images);
  markup(dom, fail, canvas.discardStagedNode);
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
    querySelector: selector => first(root, selector, true),
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
      // UTF-16 length is a lower bound on UTF-8 byte length; the ASCII fast path avoids per-character iterators and temporary strings.
      const limit = 60 * 1024;
      let bytes = encoded.length;
      if (bytes > limit) return false;
      // Avoid repeated QuickJS charCodeAt conversions for supplementary-plane strings; the host only computes the UTF-8 length of the same JSON.
      // The UTF-16 upper bound is already checked, limiting conversion to 180 KiB without exposing files, network access or other host capabilities.
      if (/[^\x00-\x7F]/.test(encoded)) bytes = utf8Length(encoded);
      if (bytes > limit) return false;
      // The initial subset reserves 4 KiB for host identity/envelope; the parent still enforces the full 64 KiB wire limit.
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
  // apply already owns the Promise boundary; invoke lifecycle methods directly to avoid adding an async stack to synchronous getters.
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
    } else if (command.type === "selectView") {
      // snapshot is a private null-prototype copy; it neither triggers plugin-installed inherited setters nor changes the authoritative revision.
      snapshot.viewId = command.viewId;
      if (typeof definition.update === "function") await definition.update({ snapshot: clone(snapshot), resources });
    } else if (command.type === "click") {
      const target = op.mapGet(nodes, command.target);
      if (suspended || !target || op.mapHas(dom.get(target).attrs, "disabled")) fail();
      // Only check connectivity here; dispatch itself retains the capture/bubble/composedPath path snapshot.
      let current = target;
      while (current && current !== root) current = dom.get(current).parent;
      if (!current) fail();
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
        // Pointer has no native default action; click retains its original threshold and rechecks connection identity after up/microtasks.
        if (input.click && connected(true) && !op.mapHas(dom.get(target).attrs, "disabled")) await dom.events.click(target);
      }
      else if (command.input.kind === "text" && (input.phase === "input" || input.phase === "change"))
        await dom.events.native(form.accept(input), input.phase, false, input.phase === "input");
      else fail();
    } else if (callback) {
      if (!mounted || typeof definition.onLiveTrigger !== "function") fail();
      // Automatic callbacks do not synthesize clicks/gestures, resume paused timers or request host focus.
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
  // Rust collects only after the apply Promise and all runnable microtasks finish, avoiding lost late emits.
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
