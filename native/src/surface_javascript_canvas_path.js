// Private bounded records for the current path; the path is not part of save/restore state, and paths spanning transforms are not yet supported.
((dom, fail, get, numbers, copy) => {
  "use strict";
  const op = dom.ops, noPrototype = Object.setPrototypeOf;
  // Wire segments and coordinates are internally constructed, prototype-free and never exposed; captured segments are shared read-only.
  const tuple = values => noPrototype(values, null);
  const point = (x, y) => tuple([x, y]);
  const append = (owner, segment) => {
    const value = get(owner), state = value.state;
    if (value.path.length >= 4096) fail();
    if (value.pathTransform) {
      for (let i = 0; i < 6; i++) if (value.pathTransform[i] !== state.transform[i]) fail();
    } else value.pathTransform = copy(state.transform);
    op.push(value.path, segment);
  };
  return {
    beginPath() { const value = get(this); value.path = op.array(); value.pathTransform = null; },
    closePath() { const value = get(this); if (value.path.length) append(this, { __proto__: null, kind: "close" }); },
    moveTo(x, y) { get(this); const point = numbers([x, y]); if (point) append(this, { __proto__: null, kind: "move", point }); },
    lineTo(x, y) { get(this); const point = numbers([x, y]); if (point) append(this, { __proto__: null, kind: "line", point }); },
    bezierCurveTo(a, b, c, d, x, y) {
      get(this); const p = numbers([a,b,c,d,x,y]);
      if (p) append(this, { __proto__: null, kind: "curve",
        points: tuple([point(p[0],p[1]),point(p[2],p[3]),point(p[4],p[5])]) });
    },
    rect(x, y, width, height) {
      get(this); const p = numbers([x,y,width,height]); if (!p) return;
      // Numeric conversion can reenter and mutate the path; reserve the entire rectangle after conversion so exceptions cannot leave a partial path.
      if (get(this).path.length > 4096 - 5) fail();
      // The private null-prototype numbers array has no iterator; do not rely on a pollutable iteration protocol.
      const a = p[0], b = p[1], w = p[2], h = p[3];
      append(this, { __proto__: null, kind: "move", point: point(a,b) });
      append(this, { __proto__: null, kind: "line", point: point(a+w,b) });
      append(this, { __proto__: null, kind: "line", point: point(a+w,b+h) });
      append(this, { __proto__: null, kind: "line", point: point(a,b+h) });
      append(this, { __proto__: null, kind: "close" });
    },
    arc(x, y, radius, start, end, ccw = false) {
      get(this); const p = numbers([x,y,radius,start,end]); if (!p) return;
      if (p[2] < 0) fail();
      append(this, { __proto__: null, kind: "arc", center: point(p[0],p[1]), radius: p[2], start: p[3], end: p[4], ccw: !!ccw });
    },
  };
})
