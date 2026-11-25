// live children 缓存只跟随直接结构变更；item 转换持旧列表，迭代恢复读取新列表。
let collectionRoot, retainedChildren;
const checkCollection = (condition, message) => { if (!condition) throw Error(message); };
function expectChildren(parent, expected) {
  const collection = parent.children;
  checkCollection(collection.length === expected.length, "children length");
  for (let i = 0; i < expected.length; i++) {
    checkCollection(collection[i] === expected[i] && collection.item(i) === expected[i], "children order");
  }
  checkCollection(collection.item(expected.length) === null, "children item bounds");
  const iterated = Array.from(collection);
  checkCollection(iterated.length === expected.length && iterated.every((value, index) => value === expected[index]), "children iteration");
}
function checkMutations(root) {
  const a = document.createElement("span"), b = document.createElement("span");
  const c = document.createElement("span"), fragment = document.createDocumentFragment();
  const other = document.createElement("div"), text = document.createTextNode("text");
  const children = root.children;
  expectChildren(root, []);
  root.append(a, text, b); expectChildren(root, [a, b]);
  text.data = "changed"; a.title = "metadata"; a.appendChild(document.createTextNode("a"));
  expectChildren(root, [a, b]);
  root.append(a); expectChildren(root, [b, a]);
  expectChildren(other, []); other.append(b);
  expectChildren(root, [a]); expectChildren(other, [b]);
  expectChildren(fragment, []); fragment.append(a, c);
  expectChildren(root, []); expectChildren(fragment, [a, c]);
  root.append(fragment, a, fragment);
  expectChildren(root, [c, a]); expectChildren(fragment, []);
  root.removeChild(c); expectChildren(root, [a]);
  a.remove(); expectChildren(root, []);
  root.replaceChildren(a, b); expectChildren(root, [a, b]); expectChildren(other, []);
  root.replaceChildren(); expectChildren(root, []);
  root.append(c); expectChildren(root, [c]);
  root.textContent = "only text"; expectChildren(root, []);
  root.innerHTML = "<span>one</span>text<span>two</span>";
  expectChildren(root, [root.firstChild, root.lastChild]);
  checkCollection(children === root.children, "collection identity changed");
  root.replaceChildren(); expectChildren(root, []);
}
function checkLiveIteration(root) {
  const a = document.createElement("span"), b = document.createElement("span"), c = document.createElement("span");
  root.append(a, b, c);
  let iterator = root.children[Symbol.iterator]();
  checkCollection(iterator.next().value === a, "iterator first");
  a.remove();
  checkCollection(iterator.next().value === c && iterator.next().done, "iterator retained old list");
  root.replaceChildren(a);
  iterator = root.children[Symbol.iterator]();
  checkCollection(iterator.next().value === a, "append iterator first");
  root.append(b);
  checkCollection(iterator.next().value === b && iterator.next().done, "iterator missed append");
  root.replaceChildren(a, b);
  iterator = root.children[Symbol.iterator](); iterator.next();
  root.replaceChildren(c, a);
  checkCollection(iterator.next().value === a, "iterator missed replace");
  root.replaceChildren();
  checkCollection(iterator.next().done, "iterator missed clear");
}
function checkItemReentry(root) {
  const a = document.createElement("span"), b = document.createElement("span"), c = document.createElement("span");
  const other = document.createElement("div"), text = document.createTextNode("text");
  for (const property of ["children", "childNodes"]) {
    for (const operation of ["remove", "replace", "move", "throw"]) {
      root.replaceChildren(a, b, text); other.replaceChildren();
      const collection = root[property];
      checkCollection(collection.length === (property === "children" ? 2 : 3), "warm item list");
      let calls = 0, caught = false, result;
      const marker = Error("index conversion");
      try {
        result = collection.item({ valueOf() {
          calls++;
          if (operation === "remove") a.remove();
          else if (operation === "replace") root.replaceChildren(c);
          else if (operation === "move") other.append(a);
          else { a.remove(); throw marker; }
          return 0;
        } });
      } catch (error) { caught = error === marker; }
      checkCollection(calls === 1 && caught === (operation === "throw"), "item conversion/exception");
      if (!caught) checkCollection(result === a, "item did not retain preconversion list");
      checkCollection(collection[0] === (operation === "replace" ? c : b), "live list did not invalidate");
    }
  }
  root.replaceChildren();
}
function checkRejectedMutations(root) {
  const a = document.createElement("div"), b = document.createElement("span"), other = document.createElement("div");
  root.append(a, b); other.append(document.createElement("span"));
  const foreign = other.firstChild;
  const reject = operation => {
    expectChildren(root, [a, b]); expectChildren(other, [foreign]); expectChildren(a, []);
    let rejected = false; try { operation(); } catch { rejected = true; }
    checkCollection(rejected, "invalid mutation accepted");
    expectChildren(root, [a, b]); expectChildren(other, [foreign]); expectChildren(a, []);
  };
  reject(() => a.append(root));
  reject(() => other.append(root));
  reject(() => root.appendChild({}));
  reject(() => root.replaceChildren(foreign, root));
  const tall = document.createElement("div");
  let parent = tall;
  for (let i = 0; i < 30; i++) {
    const next = document.createElement("div"); parent.append(next); parent = next;
  }
  reject(() => a.append(tall));
  checkCollection(tall.parentNode === null, "depth rejection attached tree");
  root.replaceChildren();
}
NeuroSurface.define({
  mount({ root }) {
    collectionRoot = root; retainedChildren = root.children;
    checkMutations(root); checkLiveIteration(root); checkItemReentry(root); checkRejectedMutations(root);
    root.textContent = "collections";
    expectChildren(root, []);
  },
  update() {
    collectionRoot.innerHTML = "<span>updated-collections</span>";
    checkCollection(collectionRoot.children === retainedChildren, "update replaced collection");
    expectChildren(collectionRoot, [collectionRoot.firstChild]);
  },
});
