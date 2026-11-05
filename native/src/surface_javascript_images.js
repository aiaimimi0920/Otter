// 只有当前宿主安装的图片能生成句柄；对象身份由私有WeakMap验证，不能伪造ID/URL。
((op, json, fail) => {
  "use strict";
  const handles = new op.Weak(), cache = new op.MapType(), freeze = Object.freeze;
  let native, disposed = false;
  return {
    bind(value) { if (native || typeof value !== "function") fail(); native = value; },
    image(id) {
      if (disposed || !native || typeof id !== "string" || id.length > 256) fail();
      if (op.mapHas(cache,id)) return op.mapGet(cache,id);
      const encoded = native(id);
      if (!encoded) fail();
      // 私有JSON数组故意无原型/迭代器，不能通过解构调用插件可变iterator。
      const size = json.decode(encoded), width = size[0], height = size[1];
      const handle = freeze({ width,height,naturalWidth:width,naturalHeight:height });
      op.weakSet(handles,handle,{ id,width,height }); op.mapSet(cache,id,handle);
      return handle;
    },
    resolve(handle) { if (disposed) fail(); return op.weakGet(handles,handle) ?? fail(); },
    dispose() { disposed = true; native = undefined; op.mapClear(cache); },
  };
})
