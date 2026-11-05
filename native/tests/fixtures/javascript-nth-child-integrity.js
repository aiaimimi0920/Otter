// 公开 intrinsic 污染不能窥视/替换私有位置索引；不污染 QuickJS 自身 JSON 数值槽。
let rootNode, first;
NeuroSurface.define({
  mount({root}) {
    rootNode = root;
    const sheet = new CSSStyleSheet();
    root.innerHTML = '<button></button> \n<button></button> \n<button></button> \n<button></button>';
    first = root.firstElementChild;
    const deny = () => { throw Error('public intrinsic dispatched'); };
    WeakMap.prototype.get = WeakMap.prototype.set = WeakMap.prototype.has = deny;
    Map.prototype.get = Map.prototype.set = Map.prototype.has = deny;
    String.prototype.trim = String.prototype.toLowerCase = String.prototype.slice = deny;
    RegExp.prototype.exec = RegExp.prototype.test = deny;
    Array.prototype.map = Array.prototype.filter = Array.prototype.indexOf = deny;
    sheet.replaceSync('button{color:blue}button:nth-child(2n + 0){color:green}');
    document.adoptedStyleSheets = [sheet];
  },
  update() { rootNode.appendChild(first); },
});
