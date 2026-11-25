// 模型与真实 child 共用：选择器边界、静态快照及插件原型污染。
let queryRoot, savedQuery, savedNode;
const checkQuery = (condition, message) => { if (!condition) throw Error(message); };
const rejectQuery = operation => {
  let rejected = false;
  try { operation(); } catch (error) {
    rejected = error.message === "Unsupported or invalid native DOM operation";
  }
  checkQuery(rejected, "selector did not reject at compile boundary");
};
function checkAttributeSelectors(root) {
  const fragment = document.createDocumentFragment(), empty = document.createElement("div");
  const valid = "data-" + "a".repeat(123), invalid = valid + "a";
  const owners = [document, root, fragment, empty];
  for (const populated of [false, true]) {
    if (populated) {
      for (const parent of [root, fragment, empty]) {
        const child = document.createElement("span");
        child.setAttribute(valid, "v"); parent.append(child);
      }
    }
    for (const owner of owners) for (const suffix of ["]", '="v"]']) {
      const selector = "[" + valid + suffix;
      const all = owner.querySelectorAll(selector);
      checkQuery(all.length === (populated ? 1 : 0), "128 character attribute boundary");
      checkQuery(owner.querySelector(selector) === all.item(0), "attribute first/all mismatch");
      for (const method of ["querySelector", "querySelectorAll"]) {
        rejectQuery(() => owner[method]("[" + invalid + suffix));
      }
    }
  }
  root.setAttribute(valid, "v");
  checkQuery(root.matches("[" + valid + "]"), "matches attribute boundary");
  checkQuery(root.firstChild.closest("[" + valid + "]") === root.firstChild, "closest self");
  for (const suffix of ["]", '="v"]']) for (const method of ["matches", "closest"]) {
    rejectQuery(() => root[method]("[" + invalid + suffix));
  }
  root.removeAttribute(valid); root.replaceChildren();
}
function checkStaticQueries(root) {
  const first = document.createElement("span"), last = document.createElement("span");
  root.append(first, last);
  const define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor;
  const setPrototype = Object.setPrototypeOf, number = Number;
  const keys = ["0", "999", "NaN", "get", "set", "enumerable", "value"];
  let hooks = 0, all, empty, conversions = 0;
  const poison = key => {
    const descriptor = Object.create(null);
    descriptor.configurable = true;
    descriptor.get = () => { hooks++; return undefined; };
    descriptor.set = () => { hooks++; };
    define(Object.prototype, key, descriptor);
  };
  try {
    for (const key of keys) poison(key);
    Object.defineProperty = Object.getOwnPropertyDescriptor = Object.setPrototypeOf = () => {
      throw Error("mutable intrinsic dispatched");
    };
    all = root.querySelectorAll("*"); empty = root.querySelectorAll("[missing]");
    checkQuery(all.length === 2 && all[0] === first && all[1] === last, "numeric setter swallowed result");
    checkQuery(all.item(0) === first && all.item(1) === last, "own item missing");
    checkQuery(all.item(999) === null && all.item(NaN) === null && empty.item(0) === null, "inherited item");
    checkQuery(all.item({ valueOf() { conversions++; first.remove(); return 0; } }) === first, "static conversion");
    checkQuery(conversions === 1 && root.firstChild === last, "conversion count or mutation");
    let numberCalls = 0, keyCalls = 0;
    globalThis.Number = () => {
      numberCalls++;
      return { [Symbol.toPrimitive]() { keyCalls++; return keyCalls === 1 ? "0" : "999"; } };
    };
    checkQuery(all.item("ignored") === first && numberCalls === 1 && keyCalls === 1, "item converted key twice");
    globalThis.Number = number;
    const item = describe(all, "item"), entry = describe(all, "0");
    checkQuery(!item.enumerable && entry.enumerable && !entry.writable, "static property flags");
    checkQuery(Object.getPrototypeOf(all) === Array.prototype && Object.isFrozen(all), "ordinary frozen Array");
    checkQuery(Array.isArray(all) && Array.from(all).length === 2, "static Array iteration");
    let visits = 0; all.forEach(() => visits++);
    checkQuery(visits === 2 && Object.keys(all).join(",") === "0,1", "enumerable static indices");
    checkQuery(hooks === 0, "prototype hook invoked");
  } finally {
    globalThis.Number = number;
    Object.defineProperty = define; Object.getOwnPropertyDescriptor = describe;
    Object.setPrototypeOf = setPrototype;
    for (const key of keys) delete Object.prototype[key];
  }
  // 缺失项必须不读取原型；正常 Array 的直接越界读取仍保留标准原型行为。
  let inheritedReads = 0;
  define(Array.prototype, "999", { configurable: true, get() { inheritedReads++; return last; } });
  try {
    checkQuery(all.item(999) === null && inheritedReads === 0, "item read inherited numeric getter");
    checkQuery(all[999] === last && inheritedReads === 1, "ordinary Array prototype changed");
  } finally { delete Array.prototype[999]; }
  root.replaceChildren();
  checkQuery(all.length === 2 && all.item(0) === first && empty.length === 0, "query became live");
  savedQuery = all; savedNode = first;
}
NeuroSurface.define({
  mount({ root }) {
    queryRoot = root;
    checkAttributeSelectors(root); checkStaticQueries(root);
    root.textContent = "queries";
  },
  update() {
    checkQuery(savedQuery.item(0) === savedNode && savedNode.parentNode === null, "snapshot changed between calls");
    queryRoot.textContent = "updated-queries";
  },
});
