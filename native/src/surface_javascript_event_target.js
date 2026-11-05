// 单文档事件 owner；私有登记/分派状态不进入 DOM 投影或远端动作合同。
((op, fail, parent) => {
  "use strict";
  const define = Object.defineProperty, PromiseType = Promise;
  const then = Function.prototype.call.bind(Promise.prototype.then);
  const constructor = op.freeze({ [Symbol.species]: PromiseType });
  let targets = new op.Weak(), events = new op.Weak(), created = 0, depth = 0, disposed = false, clock;
  const targetState = target => { if (disposed || !op.weakHas(targets, target)) fail(); return op.weakGet(targets, target); };
  const eventState = event => { if (disposed || !op.weakHas(events, event)) fail(); return op.weakGet(events, event); };
  const cancel = event => { const value = eventState(event); if (value.cancelable && !value.passive) value.canceled = true; };
  const name = value => { value = op.string(value); if (value.length > 256) fail(); return value; };
  const dictionary = value => { if (value != null && typeof value !== "object" && typeof value !== "function") fail(); };
  const options = (value, full) => {
    if (typeof value === "boolean") return { capture: value, once: false, passive: false };
    dictionary(value);
    const capture = !!value?.capture;
    if (!full) return { capture };
    const once = !!value?.once, passive = !!value?.passive, signal = value?.signal;
    // AbortSignal 还没有原生生命周期 owner；不能悄悄保存 signal 却不响应撤销。
    if (signal != null) fail();
    return { capture, once, passive };
  };
  class Event {
    constructor(type, init) {
      if (!arguments.length || disposed) fail();
      type = name(type); dictionary(init);
      const value = { type, bubbles: !!init?.bubbles, cancelable: !!init?.cancelable, composed: !!init?.composed,
        target: null, current: null, phase: 0, path: op.array(), active: false, stopped: false, immediate: false,
        canceled: false, passive: false, trusted: false, timestamp: clock ? clock() : 0 };
      op.weakSet(events, this, value);
      define(this, "isTrusted", { enumerable: true, get: () => eventState(this).trusted });
    }
    get type() { return eventState(this).type; }
    get target() { return eventState(this).target; }
    get currentTarget() { return eventState(this).current; }
    get eventPhase() { return eventState(this).phase; }
    get bubbles() { return eventState(this).bubbles; }
    get cancelable() { return eventState(this).cancelable; }
    get composed() { return eventState(this).composed; }
    get timeStamp() { return eventState(this).timestamp; }
    get defaultPrevented() { return eventState(this).canceled; }
    get cancelBubble() { return eventState(this).stopped; }
    set cancelBubble(value) { if (value) eventState(this).stopped = true; }
    get returnValue() { return !eventState(this).canceled; }
    set returnValue(value) { if (!value) cancel(this); }
    stopPropagation() { eventState(this).stopped = true; }
    stopImmediatePropagation() { const value = eventState(this); value.stopped = value.immediate = true; }
    preventDefault() { cancel(this); }
    composedPath() {
      const path = eventState(this).path, copy = [];
      for (let i = 0; i < path.length; i++) op.push(copy, path[i]);
      return copy;
    }
  }
  for (const [key, value] of [["NONE", 0], ["CAPTURING_PHASE", 1], ["AT_TARGET", 2], ["BUBBLING_PHASE", 3]]) {
    define(Event, key, { value, enumerable: true }); define(Event.prototype, key, { value, enumerable: true });
  }
  class KeyboardEvent extends Event {
    constructor(type, init) {
      super(type, init);
      const key = name(init?.key ?? ""), code = name(init?.code ?? "");
      const location = op.number(init?.location ?? 0);
      if (!op.finite(location) || location < 0 || location > 3 || location % 1) fail();
      eventState(this).keyboard = { key, code, location, ctrlKey: !!init?.ctrlKey, shiftKey: !!init?.shiftKey,
        altKey: !!init?.altKey, metaKey: !!init?.metaKey, repeat: !!init?.repeat, isComposing: !!init?.isComposing };
    }
    getModifierState(key) {
      const value = eventState(this).keyboard; if (!value) fail(); key = op.string(key);
      return key === "Control" ? value.ctrlKey : key === "Shift" ? value.shiftKey : key === "Alt" ? value.altKey : key === "Meta" ? value.metaKey : false;
    }
  }
  for (const key of ["key", "code", "location", "ctrlKey", "shiftKey", "altKey", "metaKey", "repeat", "isComposing"]) {
    define(KeyboardEvent.prototype, key, { enumerable: true, configurable: true, get() {
      const value = eventState(this).keyboard; if (!value) fail(); return value[key];
    } });
  }
  // 鼠标字段仍保存在同一事件私有状态；公开 getter 不作为宿主取消或权限依据。
  const numeric = (value, fallback, low, high, integer = false) => {
    value = op.number(value ?? fallback);
    if (!op.finite(value) || value < low || value > high || (integer && value % 1)) fail();
    return value;
  };
  class MouseEvent extends Event {
    constructor(type, init) {
      if (!arguments.length) fail();
      super(type, init);
      const relatedTarget = init?.relatedTarget ?? null;
      if (relatedTarget !== null) targetState(relatedTarget);
      define(eventState(this), "mouse", { value: { clientX: numeric(init?.clientX, 0, -131072, 131072),
        clientY: numeric(init?.clientY, 0, -131072, 131072), button: numeric(init?.button, 0, -1, 4, true),
        buttons: numeric(init?.buttons, 0, 0, 31, true), ctrlKey: !!init?.ctrlKey, shiftKey: !!init?.shiftKey,
        altKey: !!init?.altKey, metaKey: !!init?.metaKey, relatedTarget } });
    }
    getModifierState(key) {
      const state = eventState(this); if (!op.own(state, "mouse")) fail(); const value = state.mouse; key = op.string(key);
      return key === "Control" ? value.ctrlKey : key === "Shift" ? value.shiftKey : key === "Alt" ? value.altKey : key === "Meta" ? value.metaKey : false;
    }
  }
  class PointerEvent extends MouseEvent {
    constructor(type, init) {
      if (!arguments.length) fail();
      super(type, init);
      define(eventState(this), "pointer", { value: { pointerId: numeric(init?.pointerId, 0, 0, 2147483647, true),
        pointerType: name(init?.pointerType ?? ""), isPrimary: !!init?.isPrimary } });
    }
  }
  for (const [Type, family, keys] of [[MouseEvent, "mouse", ["clientX", "clientY", "button", "buttons", "ctrlKey", "shiftKey", "altKey", "metaKey", "relatedTarget"]],
    [PointerEvent, "pointer", ["pointerId", "pointerType", "isPrimary"]]]) {
    for (const key of keys) define(Type.prototype, key, { enumerable: true, configurable: true, get() {
      const state = eventState(this); if (!op.own(state, family)) fail(); return state[family][key];
    } });
  }
  const erase = (list, entry) => {
    entry.removed = true;
    const index = op.indexOf(list, entry); if (index >= 0) op.remove(list, index);
  };
  // native 入口由原 Engine 的 job 边界直接调用，不把 apply/invoke 栈叠到同步 CSS/几何。
  const dispatch = (target, event, trusted = true) => {
    if (trusted) { const input = target; target = input.node; event = input.pointer ? new PointerEvent(input.type, input.pointer) : input.keyboard ? new KeyboardEvent(input.type, input.keyboard) : new Event(input.type, { bubbles: true, cancelable: input.cancelable, composed: input.composed }); }
    targetState(target); const value = eventState(event);
    if (value.active || depth >= 32) fail();
    const path = op.array();
    for (let node = target; node; node = parent(node)) { if (path.length >= 32) fail(); op.push(path, node); }
    value.trusted = trusted; value.target = target; value.path = path; value.active = true; depth++;
    try {
      // 路径在首个回调前固定；每个目标/阶段分别取快照，后阶段可见新登记。
      for (let step = 0; step < path.length * 2; step++) {
        if (value.stopped) break;
        const capture = step < path.length, index = capture ? path.length - step - 1 : step - path.length;
        if (!capture && index && !value.bubbles) continue;
        const current = path[index], list = targetState(current), snapshot = op.map(list, entry => entry), args = op.array();
        value.current = current; value.phase = index ? capture ? 1 : 3 : 2; op.push(args, event);
        for (let i = 0; i < snapshot.length; i++) {
          const entry = snapshot[i];
          if (entry.removed || entry.type !== value.type || entry.capture !== capture) continue;
          // 在回调前删除 once，递归分派也不能再见到它；移除标记影响旧快照。
          if (entry.once) erase(list, entry);
          value.passive = entry.passive;
          try {
            const object = typeof entry.callback !== "function", callback = object ? entry.callback.handleEvent : entry.callback;
            if (typeof callback !== "function") fail();
            op.apply(callback, object ? entry.callback : current, args);
          } finally { value.passive = false; }
          if (value.immediate) break;
        }
      }
      return !value.canceled;
    } finally {
      // 保留原宿主的异常终止边界，不用吞 listener 异常伪造浏览器 ErrorEvent 成功。
      value.current = null; value.phase = 0; value.path = op.array();
      value.active = value.stopped = value.immediate = value.passive = false; depth--;
    }
  };
  class EventTarget {
    constructor() {
      if (disposed || created >= 2048) fail();
      created++; op.weakSet(targets, this, op.array());
    }
    addEventListener(type, callback, init) {
      if (arguments.length < 2) fail();
      const list = targetState(this); type = name(type);
      if (callback != null && typeof callback !== "function" && typeof callback !== "object") fail();
      const settings = options(init, true);
      if (callback == null) return;
      for (let i = 0; i < list.length; i++) if (list[i].type === type && list[i].callback === callback && list[i].capture === settings.capture) return;
      if (list.length >= 16) fail();
      op.push(list, { type, callback, capture: settings.capture, once: settings.once, passive: settings.passive, removed: false });
    }
    removeEventListener(type, callback, init) {
      if (arguments.length < 2) fail();
      if (callback != null && typeof callback !== "function" && typeof callback !== "object") fail();
      const list = targetState(this); type = name(type); const capture = options(init, false).capture;
      for (let i = 0; i < list.length; i++) if (list[i].type === type && list[i].callback === callback && list[i].capture === capture) { erase(list, list[i]); return; }
    }
    dispatchEvent(event) { return dispatch(this, event, false); }
  }
  const native = (target, type, cancelable, composed = true, keyboard, pointer) => {
      // 非 thenable 私有 carrier，既不读取 Node.then，也不经过插件的 Promise species/then。
      const value = op.record(); value.node = target; value.type = type; value.cancelable = cancelable; value.composed = composed; value.keyboard = keyboard; value.pointer = pointer;
      const pending = new PromiseType(resolve => resolve(value));
      define(pending, "constructor", { value: constructor });
      const result = then(pending, dispatch);
      define(result, "constructor", { value: PromiseType });
      return result;
  };
  return {
    Event, KeyboardEvent, MouseEvent, PointerEvent, EventTarget, native, click: target => native(target, "click", true),
    pointer(target, input) {
      const init = op.record(), flags = input.modifiers, terminal = input.phase === "pointerup" || input.phase === "pointercancel";
      init.bubbles = init.composed = true; init.cancelable = input.phase !== "pointercancel";
      init.clientX = input.client[0]; init.clientY = input.client[1];
      init.button = input.phase === "pointermove" || input.phase === "pointercancel" ? -1 : 0; init.buttons = terminal ? 0 : 1;
      init.pointerId = 1; init.pointerType = "mouse"; init.isPrimary = true;
      init.ctrlKey = !!(flags & 1); init.shiftKey = !!(flags & 2); init.altKey = !!(flags & 4); init.metaKey = !!(flags & 8);
      return native(target, input.phase, init.cancelable, true, undefined, init);
    },
    keyboard(target, input) {
      const flags = input.flags, init = op.record();
      init.bubbles = init.cancelable = init.composed = true;
      init.key = "Enter"; init.code = flags & 64 ? "NumpadEnter" : "Enter"; init.location = flags & 64 ? 3 : 0;
      init.ctrlKey = !!(flags & 4); init.shiftKey = !!(flags & 8); init.altKey = !!(flags & 16); init.metaKey = !!(flags & 32);
      init.repeat = !!(flags & 2); init.isComposing = false;
      return native(target, input.phase, true, true, init);
    },
    bindClock(value) { if (clock || typeof value !== "function") fail(); clock = value; },
    dispose() { disposed = true; targets = new op.Weak(); events = new op.Weak(); clock = undefined; },
  };
})
