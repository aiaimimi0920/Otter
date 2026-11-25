// Only images installed by the current host can produce handles; a private WeakMap verifies object identity, preventing forged IDs/URLs.
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
      // Private JSON arrays intentionally have no prototype/iterator; do not invoke plugin-mutable iterators through destructuring.
      const size = json.decode(encoded), width = size[0], height = size[1];
      const handle = freeze({ width,height,naturalWidth:width,naturalHeight:height });
      op.weakSet(handles,handle,{ id,width,height }); op.mapSet(cache,id,handle);
      return handle;
    },
    resolve(handle) { if (disposed) fail(); return op.weakGet(handles,handle) ?? fail(); },
    dispose() { disposed = true; native = undefined; op.mapClear(cache); },
  };
})
