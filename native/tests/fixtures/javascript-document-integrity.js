// 同一真实引擎的树顺序/私有状态回归；故意保留 intrinsic 覆盖到 collect/update/dispose。
let label;
NeuroSurface.define({
  mount({ root }) {
    let parent = root;
    for (let i = 0; i < 30; i++) { const node = document.createElement("div"); parent.appendChild(node); parent = node; }
    const ancestor = document.createElement("div"), descendant = document.createElement("span");
    ancestor.appendChild(descendant); parent.append(ancestor, descendant);
    if (parent.childNodes.length !== 2 || descendant.parentNode !== parent) throw Error("final depth32 move rejected");
    root.replaceChildren();
    const fragment = document.createDocumentFragment(), a = document.createElement("span"), b = document.createElement("span");
    a.textContent = "a"; b.textContent = "b"; fragment.append(a, b);
    root.append(fragment, a, fragment);
    if (root.textContent !== "ba" || root.firstChild !== b || fragment.childNodes.length !== 0) throw Error("fragment append order");
    fragment.append(a, b); root.replaceChildren(fragment, a, fragment);
    if (root.textContent !== "ba" || root.lastChild !== a) throw Error("fragment replace order");
    fragment.append(a, b); root.replaceChildren(b, fragment);
    if (root.textContent !== "ba" || root.lastChild !== a) throw Error("individual before fragment");
    const weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set;
    const mapGet = Map.prototype.get, mapSet = Map.prototype.set;
    let leaked = false;
    const capture = value => { if (value && value.attrs && value.children && value.key) leaked = true; return value; };
    WeakMap.prototype.get = function (key) { return capture(weakGet.call(this, key)); };
    WeakMap.prototype.set = function (key, value) { capture(value); return weakSet.call(this, key, value); };
    Map.prototype.get = function (key) { return capture(mapGet.call(this, key)); };
    Map.prototype.set = function (key, value) { capture(value); return mapSet.call(this, key, value); };
    Map.prototype.forEach = Array.prototype.map = Array.prototype.filter = function () { throw Error("private collection dispatched"); };
    Array.prototype[Symbol.iterator] = function () { throw Error("private iterator dispatched"); };
    Object.defineProperty(Object.prototype, "childNodes", { configurable: true, set() { leaked = true; } });
    Object.defineProperty(Object.prototype, "elements", { configurable: true, set() { leaked = true; } });
    label = document.createElement("span"); label.textContent = "integrity";
    delete Object.prototype.childNodes;
    delete Object.prototype.elements;
    root.replaceChildren(label);
    const button = document.createElement("button"); button.textContent = "safe"; root.appendChild(button);
    label.dataset.ref = "label"; label.classList.add("safe");
    if (root.textContent !== "integritysafe" || root.children.length !== 2 || leaked) throw Error("private tree leaked");
  },
  update() { label.textContent = "updated-integrity"; },
});
