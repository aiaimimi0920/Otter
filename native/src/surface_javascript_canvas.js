// 文档 Canvas identity、位图尺寸重置和 native 提交；不公开 bridge，不增加文件/网络入口。
((dom, fail, values, json, makeContext, makePath, images) => {
  "use strict";
  const op = dom.ops, instances = new op.MapType(), contexts = makeContext(dom,fail,values,json,makePath,images);
  const originalSet = dom.classes.Element.prototype.setAttribute, originalRemove = dom.classes.Element.prototype.removeAttribute;
  const descriptor = Object.getOwnPropertyDescriptor, own = Object.hasOwn;
  let native, disposed = false;
  const get = node => {
    if (disposed || dom.get(node).tag !== "canvas") fail();
    const id = dom.get(node).key;
    if (!op.mapHas(instances,id)) {
      if (op.mapSize(instances) >= 8) fail();
      const owner = { node, id, width:300, height:150, reset:true, context:null };
      contexts.reset(owner); op.mapSet(instances,id,owner);
    }
    return op.mapGet(instances,id);
  };
  const flush = owner => {
    if (!owner.reset && owner.commands.length === 0) return;
    if (!native || !native(json.encode({ id:owner.id, size:[owner.width,owner.height], reset:owner.reset, commands:owner.commands }))) fail();
    owner.reset = false; owner.commands = op.array();
  };
  const resize = (node,name,value,remove) => {
    const owner = get(node), fallback = name === "width" ? 300 : 150;
    const match = value === null ? null : op.exec(/^[\t\n\f\r ]*\+?(\d+)/,value);
    const next = match ? op.number(match[1]) : fallback;
    const width = name === "width" ? next : owner.width, height = name === "height" ? next : owner.height;
    if (width > 4096 || height > 4096 || width * height > 4194304) fail();
    let total = width * height;
    op.mapEach(instances,other => { if (other !== owner) total += other.width * other.height; });
    if (total > 8388608) fail();
    const attrs = dom.get(node).attrs;
    if (!remove && !op.mapHas(attrs,name) && op.mapSize(attrs) >= 64) fail();
    dom.changed();
    if (remove) op.mapDelete(attrs,name); else op.mapSet(attrs,name,value);
    owner[name] = next; owner.reset = true; contexts.reset(owner);
  };
  Object.defineProperties(dom.classes.HTMLCanvasElement.prototype, {
    width: { get() { return get(this).width; }, set(value) { const n = op.number(value) >>> 0; resize(this,"width",op.string(n),false); } },
    height: { get() { return get(this).height; }, set(value) { const n = op.number(value) >>> 0; resize(this,"height",op.string(n),false); } },
    getContext: { value(type,options) {
      const owner = get(this); if (op.string(type) !== "2d") return null;
      if (options !== undefined && options !== null) fail();
      if (!owner.context) owner.context = contexts.create(owner); return owner.context;
    } },
    setAttribute: { value(name,value) {
      name = op.lower(op.string(name));
      if (name === "width" || name === "height") { value = op.string(value); if (value.length > 4096) fail(); resize(this,name,value,false); }
      else op.apply(originalSet,this,[name,value]);
    } },
    removeAttribute: { value(name) {
      name = op.lower(op.string(name));
      if (name === "width" || name === "height") resize(this,name,null,true);
      else op.apply(originalRemove,this,[name]);
    } },
  });
  Object.defineProperty(globalThis,"CanvasRenderingContext2D",{ value:contexts.CanvasRenderingContext2D });
  return {
    begin: contexts.begin,
    dimensions(node) { const owner = get(node); return [owner.width,owner.height]; },
    serviceFrames(handles) {
      const owners = op.array(), frames = op.array();
      const length = handles.length;
      if (length < 1 || length > 4) fail();
      for (let i = 0; i < length; i++) {
        const item = descriptor(handles,i);
        if (!item || !own(item,"value")) fail();
        const owner = get(item.value);
        if (!owner.width || !owner.height || op.indexOf(owners,owner) >= 0) fail();
        op.push(owners,owner);
      }
      // 同步提交调用时的命令；后续JS修改只进入新队列，不能改变在途快照。
      for (let i = 0; i < owners.length; i++) {
        const owner = owners[i]; flush(owner);
        op.push(frames,{ id:owner.id,width:owner.width,height:owner.height,png:"" });
      }
      return frames;
    },
    bind(value) { if (native || typeof value !== "function") fail(); native = value; },
    collect() {
      const frames = op.array();
      op.mapEach(instances,owner => {
        flush(owner);
        let node = owner.node;
        while (node && node !== dom.root) node = dom.get(node).parent;
        if (node) op.push(frames,{ id:owner.id,width:owner.width,height:owner.height,png:"" });
      });
      return frames;
    },
    dispose() { disposed = true; op.mapClear(instances); native = undefined; },
  };
})
