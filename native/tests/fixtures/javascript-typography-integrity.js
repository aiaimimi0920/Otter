// 故意污染可变 intrinsic，私有 typography 不得派发或泄露解析中间对象。
let saved;
NeuroSurface.define({mount({root}) {
  saved = root;
  String.prototype.trim = String.prototype.toLowerCase = String.prototype.split = () => {throw Error('string intrinsic')};
  RegExp.prototype.exec = () => {throw Error('regexp intrinsic')};
  Array.prototype.join = Array.prototype[Symbol.iterator] = () => {throw Error('array intrinsic')};
  Map.prototype.get = Map.prototype.set = Map.prototype.forEach = () => {throw Error('map intrinsic')};
  Number.isFinite = parseFloat = Number = () => {throw Error('number intrinsic')};
  root.style.font = 'BOLD 18PX / 1.25 "Segoe UI",sans-serif';
  if (root.style.font !== '700 18px/1.25 "Segoe UI",sans-serif') throw Error('polluted typography');
}, update() { saved.style.font = '12px Consolas,monospace'; }});
