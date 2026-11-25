// 当前路径的私有有界记录；路径不在 save/restore 状态中，跨变换路径尚未准入。
((dom, fail, get, numbers, copy) => {
  "use strict";
  const op = dom.ops, noPrototype = Object.setPrototypeOf;
  // wire 段和坐标均由内部构造、无原型且不会外露；捕获后的段只读共享。
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
      // 数值转换可重入修改路径；转换完成后预留整个矩形，异常不能留下半条路径。
      if (get(this).path.length > 4096 - 5) fail();
      // numbers 的私有无原型数组没有 iterator，不能依赖可污染的迭代协议。
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
