// 私有协议只编码自有数据，不能调用插件安装的 inherited toJSON 或 getter。
// 对外 snapshot 仍是普通 JS 副本；emit payload 单独保留标准 JSON 序列化语义。
(() => {
  "use strict";
  const parse = JSON.parse.bind(JSON), stringify = JSON.stringify.bind(JSON);
  const descriptors = Object.getOwnPropertyDescriptors, own = Object.hasOwn;
  const create = Object.create, setPrototype = Object.setPrototypeOf, isArray = Array.isArray;
  const finite = Number.isFinite;
  const copy = value => {
    const pending = setPrototype([], null);
    const prepare = (value, depth) => {
      if (depth > 128) throw new Error("Native JSON depth exceeded");
      if (value === null || typeof value === "string" || typeof value === "boolean") return value;
      if (typeof value === "number" && finite(value)) return value;
      if (typeof value !== "object") throw new Error("Invalid private native JSON value");
      const output = isArray(value) ? setPrototype([], null) : create(null);
      pending[pending.length] = { value, output, depth }; return output;
    };
    const output = prepare(value, 0);
    while (pending.length) {
      const item = pending[pending.length - 1]; pending.length--;
      // 一次native调用取得自有描述符，避免完整帧每个字段跨JS/native取描述符。
      // 描述符表先去原型；仍只复制可枚举字符串数据属性，不调用getter/toJSON。
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
  return { encode, decode: value => copy(parse(value)), clone: value => parse(encode(value)), payload: stringify };
})()
