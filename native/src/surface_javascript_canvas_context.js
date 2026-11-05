// Canvas2D 的状态、保存栈和绘制记录；只交私有 native raster，不把业务图表翻译为 Rust。
((dom, fail, values, json, makePath, images) => {
  "use strict";
  const op = dom.ops, states = new op.Weak(), define = Object.defineProperties, seal = Object.seal, from = Array.from;
  const copy = list => op.map(list, value => value);
  const defaults = () => ({ transform: [1,0,0,1,0,0], fillStyle: "#000000", strokeStyle: "#000000", globalAlpha: 1,
    lineWidth: 1, lineCap: "butt", lineJoin: "miter", miterLimit: 10, lineDashOffset: 0, dash: op.array(), font: "10px sans-serif", textAlign: "start", textBaseline: "alphabetic", imageSmoothingEnabled: true });
  const get = owner => op.weakGet(states, owner) ?? fail();
  const numbers = input => {
    const output = op.array();
    for (let i = 0; i < input.length; i++) {
      const value = op.number(input[i]); if (!op.finite(value)) return null;
      if (op.abs(value) > 1048576) fail(); op.push(output, value);
    }
    return output;
  };
  let commands = 0;
  const record = (owner, value) => {
    const target = get(owner);
    if (++commands > 8192 || target.commands.length >= 8192) fail();
    op.push(target.commands, value);
  };
  const state = (owner, stroke) => {
    const s = get(owner).state;
    return { transform: copy(s.transform), color: stroke ? s.strokeStyle : s.fillStyle, alpha: s.globalAlpha,
      width: s.lineWidth, cap: s.lineCap, join: s.lineJoin, miter: s.miterLimit, dash: copy(s.dash), dashOffset: s.lineDashOffset,
      font: s.font, align: s.textAlign, baseline: s.textBaseline };
  };
  const path = (owner, stroke, rule) => {
    const value = get(owner);
    if (rule !== "nonzero" && rule !== "evenodd") fail();
    if (value.pathTransform) for (let i = 0; i < 6; i++) if (value.pathTransform[i] !== value.state.transform[i]) fail();
    record(owner, { kind: "path", path: json.clone(value.path), state: state(owner, stroke), stroke, evenodd: rule === "evenodd" });
  };
  const key = {};
  class CanvasRenderingContext2D {
    constructor(token, owner) { if (token !== key) fail(); op.weakSet(states, this, owner); seal(this); }
    get canvas() { return get(this).node; }
    save() {
      const value = get(this); if (value.stack.length >= 32) fail();
      op.push(value.stack, json.clone(value.state));
    }
    restore() { const value = get(this); if (value.stack.length) { value.state = value.stack[value.stack.length - 1]; value.stack.length--; } }
    setTransform(a,b,c,d,e,f) { const owner = get(this), value = numbers([a,b,c,d,e,f]); if (value) owner.state.transform = value; }
    resetTransform() { get(this).state.transform = [1,0,0,1,0,0]; }
    clearRect(x,y,w,h) {
      get(this); const rect = numbers([x,y,w,h]); if (!rect) return;
      const transform = copy(get(this).state.transform);
      if (transform[1] !== 0 || transform[2] !== 0) fail();
      record(this, { kind: "clear", rect, transform });
    }
    fillRect(x,y,w,h) { get(this); const rect = numbers([x,y,w,h]); if (rect) record(this, { kind: "rect", rect, state: state(this,false), stroke:false }); }
    strokeRect(x,y,w,h) { get(this); const rect = numbers([x,y,w,h]); if (rect) record(this, { kind: "rect", rect, state: state(this,true), stroke:true }); }
    drawImage(image,...args) {
      const owner = get(this), input = images.resolve(image);
      if (args.length !== 2 && args.length !== 4 && args.length !== 8) fail();
      const n = numbers(args); if (!n) return;
      const source = n.length === 8 ? [n[0],n[1],n[2],n[3]] : [0,0,input.width,input.height];
      const destination = n.length === 2 ? [n[0],n[1],input.width,input.height] : n.length === 4 ? n : [n[4],n[5],n[6],n[7]];
      record(this,{ kind:"image",id:input.id,source,destination,transform:copy(owner.state.transform),alpha:owner.state.globalAlpha,smooth:owner.state.imageSmoothingEnabled });
    }
    fill(rule = "nonzero") { path(this,false,rule); }
    stroke() { if (arguments.length) fail(); path(this,true,"nonzero"); }
    fillText(value,x,y) {
      get(this);
      if (arguments.length > 3) fail();
      const point = numbers([x,y]), text = op.string(value); if (!point) return;
      if (text.length > 2048 || op.index(text,"\0") >= 0) fail();
      record(this, { kind: "text", text, point, state: state(this,false) });
    }
    setLineDash(input) {
      const owner = get(this);
      if (!op.isArray(input) || input.length > 32) fail();
      const list = numbers(input); if (!list) return;
      for (let i = 0; i < list.length; i++) if (list[i] < 0) return; else if (list[i] > 8192) fail();
      const result = copy(list); if (list.length % 2) for (let i = 0; i < list.length; i++) op.push(result,list[i]);
      owner.state.dash = result;
    }
    getLineDash() { return from(get(this).state.dash); }
  }
  const methods = makePath(dom,fail,get,numbers,copy);
  for (const name of Object.keys(methods)) Object.defineProperty(CanvasRenderingContext2D.prototype,name,{ value:methods[name] });
  const property = (name, validate) => define(CanvasRenderingContext2D.prototype, { [name]: {
    get() { return get(this).state[name]; },
    set(value) { const owner = get(this), normalized = validate(value); if (normalized !== undefined) owner.state[name] = normalized; },
  } });
  const color = input => {
    if (typeof input !== "string") fail();
    try { const result = values.expand("color",input)[0].value; if (result === "currentcolor") fail(); return result; } catch { return undefined; }
  };
  property("fillStyle",color); property("strokeStyle",color);
  property("globalAlpha",value => { const n = op.number(value); if (op.finite(n) && n >= 0 && n <= 1) return n; });
  property("imageSmoothingEnabled",value => !!value);
  for (const name of ["lineWidth","miterLimit"]) property(name,value => {
    const n = op.number(value); if (!op.finite(n) || n <= 0) return; if (n < 0.001 || n > 8192) fail(); return n;
  });
  property("lineDashOffset",value => { const n = numbers([value]); return n ? n[0] : undefined; });
  for (const [name,choices] of [["lineCap",["butt","round","square"]],["lineJoin",["miter","round","bevel"]],
    ["textAlign",["left","right","center","start","end"]],["textBaseline",["alphabetic","top","middle","bottom"]]])
    property(name,value => op.indexOf(choices,value) >= 0 ? value : undefined);
  property("font",value => {
    const text = op.string(value), match = op.exec(/^(\d+(?:\.\d+)?|\.\d+)px ([^\x00-\x1f]+)$/,text);
    if (!match || text.length > 256 || op.number(match[1]) < 1 || op.number(match[1]) > 256) return;
    return text;
  });
  return { CanvasRenderingContext2D, defaults, begin() { commands = 0; },
    create(owner) { return new CanvasRenderingContext2D(key,owner); },
    reset(owner) { owner.state = defaults(); owner.stack = op.array(); owner.path = op.array(); owner.pathTransform = null; owner.commands = op.array(); },
  };
})
