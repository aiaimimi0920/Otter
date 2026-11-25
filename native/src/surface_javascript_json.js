// The private protocol encodes only own data and must not invoke plugin-installed inherited toJSON methods or getters.
// Public snapshots remain ordinary JS copies; emit payloads separately retain standard JSON serialization semantics.
(() => {
  "use strict";
  const parse = JSON.parse.bind(JSON), stringify = JSON.stringify.bind(JSON);
  const descriptors = Object.getOwnPropertyDescriptors, own = Object.hasOwn;
  const create = Object.create, setPrototype = Object.setPrototypeOf, isArray = Array.isArray;
  const finite = Number.isFinite;
  const copy = (value, isPrivateCanvasPath) => {
    const pending = setPrototype([], null);
    const prepare = (value, depth) => {
      if (depth > 128) throw new Error("Native JSON depth exceeded");
      if (value === null || typeof value === "string" || typeof value === "boolean") return value;
      if (typeof value === "number" && finite(value)) return value;
      if (typeof value !== "object") throw new Error("Invalid private native JSON value");
      // Only the private Canvas snapshot table may bypass copying; other branches still validate own descriptors item by item.
      if (isPrivateCanvasPath && isPrivateCanvasPath(value)) return value;
      const output = isArray(value) ? setPrototype([], null) : create(null);
      pending[pending.length] = { value, output, depth }; return output;
    };
    const output = prepare(value, 0);
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      // Obtain own descriptors in one native call, avoiding JS/native crossings for every field of a complete frame.
      // Remove the descriptor table's prototype first; still copy only enumerable string-keyed data properties without invoking getters/toJSON.
      const properties = setPrototype(descriptors(item.value), null);
      for (const name in properties) {
        const property = properties[name];
        if (!property.enumerable) continue;
        if (!own(property, "value")) throw new Error("Invalid private native JSON property");
        item.output[name] = prepare(property.value, item.depth + 1);
      }
    }
    return output;
  };
  const encode = value => stringify(copy(value));
  return { encode, encodeCanvas: (value, isPrivateCanvasPath) => stringify(copy(value, isPrivateCanvasPath)),
    decode: value => copy(parse(value)), clone: value => parse(encode(value)), payload: stringify };
})()
