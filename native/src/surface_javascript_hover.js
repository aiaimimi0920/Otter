// Retain only the last dispatched hover path (at most 32 nodes), not a second DOM or native geometry.
((dom, fail, sheets) => {
  "use strict";
  const { ops: op, root } = dom;
  let previous = op.array();
  // Use only the previous target's current private parent chain; detached nodes do not match, and moves do not retain old ancestor style state.
  // Do not read computed (avoiding selector->hover->computed recursion) or public properties that plugins can override.
  const matches = expected => {
    let found = false, depth = 0;
    for (let node = previous[0]; node; node = dom.get(node).parent) {
      if (++depth > 32) fail();
      if (node === expected) found = true;
      if (node === root) return found;
    }
    return false;
  };
  const pathFor = target => {
    const path = op.array();
    for (let node = target; node; node = dom.get(node).parent) {
      if (path.length >= 32 || sheets.computed(node).display === "none") fail();
      op.push(path, node);
    }
    if (target && path[path.length - 1] !== root) fail();
    return path;
  };
  const send = (target, type, related, input, bubbles, cancelable) => {
    const init = op.record(), flags = input.modifiers;
    init.bubbles = bubbles; init.cancelable = cancelable; init.composed = bubbles;
    init.clientX = input.client[0]; init.clientY = input.client[1];
    init.button = -1; init.buttons = 0; init.pointerId = 1;
    init.pointerType = "mouse"; init.isPrimary = true; init.relatedTarget = related;
    init.ctrlKey = !!(flags & 1); init.shiftKey = !!(flags & 2);
    init.altKey = !!(flags & 4); init.metaKey = !!(flags & 8);
    return dom.events.native(target, type, cancelable, true, undefined, init);
  };
  return {
    matches,
    async accept(input) {
      const target = input.target === null ? null : op.mapGet(dom.nodes, input.target);
      if (input.target !== null && !target) fail();
      const next = pathFor(target), old = previous, source = old[0] ?? null;
      let same = next.length === old.length;
      for (let i = 0; same && i < next.length; i++) same = next[i] === old[i];
      // Fix the path and new physical position before callbacks; tree mutations in callbacks do not rewrite this crossing event's targets.
      previous = next;
      if (!same) {
        dom.changed();
        let common = 0;
        while (common < old.length && common < next.length && old[old.length - 1 - common] === next[next.length - 1 - common]) common++;
        if (source) await send(source, "pointerout", target, input, true, true);
        for (let i = 0; i < old.length - common; i++) await send(old[i], "pointerleave", target, input, false, false);
        if (target) await send(target, "pointerover", source, input, true, true);
        for (let i = next.length - common - 1; i >= 0; i--) await send(next[i], "pointerenter", source, input, false, false);
      }
      if (target) await send(target, "pointermove", null, input, true, true);
    },
    clear() { if (previous.length) { previous = op.array(); dom.changed(); } },
  };
})
