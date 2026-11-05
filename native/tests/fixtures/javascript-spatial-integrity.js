// 所有 parser/反射/投影均不得分派到插件污染后的 collection、字符串或数值 intrinsic。
let spatialIntegrityRoot;
NeuroSurface.define({ mount({ root }) {
  spatialIntegrityRoot = root; root.style.position = "relative"; root.innerHTML = "<div></div>";
  const node = root.firstChild; node.style.cssText = "position:absolute;inset:0;overflow:hidden";
  Map.prototype.get = Map.prototype.set = Map.prototype.forEach = () => { throw Error("map"); };
  Object.getPrototypeOf(new Map().entries()).next = () => { throw Error("map next"); };
  Map.prototype.entries = Map.prototype[Symbol.iterator] = () => { throw Error("map entries"); };
  Array.prototype.join = Array.prototype[Symbol.iterator] = () => { throw Error("array"); };
  String.prototype.split = String.prototype.trim = String.prototype.toLowerCase = () => { throw Error("string"); };
  RegExp.prototype.exec = () => { throw Error("regexp"); };
  globalThis.parseFloat = globalThis.Number = () => { throw Error("number"); };
  Object.prototype.toJSON = () => ({ stolen: true });
  Object.defineProperty(Object.prototype, "names", { set() { throw Error("prototype setter"); }, configurable: true });
  Object.defineProperty(Object.prototype, "quad", { get() { throw Error("prototype quad"); }, configurable: true });
  node.style.inset = "1px 2px";
  if (node.style.inset !== "1px 2px" || node.style.overflow !== "hidden") throw Error("reflection");
}, update() {
  spatialIntegrityRoot.firstChild.style.inset = "3px 4px";
  if (spatialIntegrityRoot.firstChild.style.inset !== "3px 4px") throw Error("cross-operation");
} });
