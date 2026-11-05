// 污染跨 collect/update/dispose；新 parser/展开/反射只能调用预捕获 intrinsic。
let host, sheet, leaks = 0;
NeuroSurface.define({
  mount({ root }) {
    host = root; root.innerHTML = '<button>safe</button>';
    sheet = new CSSStyleSheet(); document.adoptedStyleSheets = [sheet];
    WeakMap.prototype.get = WeakMap.prototype.set = Map.prototype.get = Map.prototype.set = Map.prototype.forEach = () => { throw Error('mutable map'); };
    Array.prototype.map = Array.prototype.filter = Array.prototype[Symbol.iterator] = () => { throw Error('mutable array'); };
    String.prototype.trim = String.prototype.toLowerCase = String.prototype.split = String.prototype.slice = String.prototype.indexOf = () => { throw Error('mutable string'); };
    RegExp.prototype.exec = RegExp.prototype.test = () => { throw Error('mutable regexp'); };
    globalThis.Number = globalThis.parseFloat = () => { throw Error('mutable number'); };
    Math.fround = () => { throw Error('mutable fround'); };
    const names = ['gridRow','rowGap','gridTemplateColumns','value','priority','first','second','name','layout'];
    for (let i=0;i<names.length;i++) {
      const descriptor = Object.create(null); descriptor.configurable = true; descriptor.set = () => { leaks++; };
      Object.defineProperty(Object.prototype,names[i],descriptor);
    }
    sheet.replaceSync(':root{display:grid;grid-template-columns:repeat(2,minmax(0,.75fr));gap:2px 4px;place-items:center}');
    root.firstChild.style.gridColumn = '+2 span'; root.firstChild.style.gridRow = '+2 / +3';
    root.style.gap = '3px 5px';
    if (root.style.gap !== '3px 5px' || root.firstChild.style.gridColumn !== 'span 2 / auto') throw Error('reflection');
    let failed=false; try { root.style.gap='2px nope'; } catch { failed=true; }
    if (!failed || leaks) throw Error('validation leaked');
  },
  update() {
    if (leaks) throw Error('collect leaked');
    sheet.replaceSync(':root{display:grid;grid-template-columns:40px 1fr;place-items:start end}');
    host.style.removeProperty('gap'); host.firstChild.style.gridColumn = '1 / -1';
  },
  dispose() { if (leaks) throw Error('private state leaked'); },
});
