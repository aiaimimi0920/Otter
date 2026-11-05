// 字体准入与跨帧继承不能依赖插件可替换的 String/RegExp/Array/Map 方法。
let host;
NeuroSurface.define({
  mount({root}) {
    host = root;
    String.prototype.trim = String.prototype.toLowerCase = String.prototype.split = () => { throw Error('polluted string'); };
    RegExp.prototype.exec = RegExp.prototype.test = () => { throw Error('polluted regexp'); };
    Array.prototype.join = Array.prototype.push = () => { throw Error('polluted array'); };
    Map.prototype.get = Map.prototype.set = () => { throw Error('polluted map'); };
    root.style.fontFamily = 'MissingOtterFamily, "Microsoft YaHei", monospace';
    root.textContent = 'safe';
  },
  update() { host.style.fontFamily = 'Consolas,monospace'; },
});
