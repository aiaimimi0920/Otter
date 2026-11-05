// 污染公开 intrinsic 后四边展开/反射仍由原捕获操作完成。
let saved;
NeuroSurface.define({mount({root}) {
  saved = root;
  String.prototype.trim = String.prototype.toLowerCase = String.prototype.split = () => {throw Error('string intrinsic')};
  RegExp.prototype.exec = () => {throw Error('regexp intrinsic')};
  Array.prototype.join = Array.prototype[Symbol.iterator] = () => {throw Error('array intrinsic')};
  Map.prototype.get = Map.prototype.set = Map.prototype.forEach = () => {throw Error('map intrinsic')};
  Number.isFinite = parseFloat = Number = () => {throw Error('number intrinsic')};
  root.style.boxSizing = 'border-box'; root.style.padding = '1PX 2PX 3PX 4PX';
  if (root.style.padding !== '1px 2px 3px 4px') throw Error('polluted padding');
}, update() { saved.style.paddingLeft = '12px'; }});
