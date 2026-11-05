// 保持 intrinsic/prototype 污染跨越 collect/update/dispose，私有状态和准入不能随之改写。
let sheet, rootNode, leaks = 0;
NeuroSurface.define({
  mount({ root }) {
    rootNode = root; root.textContent = 'integrity';
    sheet = new CSSStyleSheet(); sheet.replaceSync('*{box-sizing:border-box}:root{background:green}');
    document.adoptedStyleSheets = [sheet];
    WeakMap.prototype.get = WeakMap.prototype.set = Map.prototype.get = Map.prototype.set = Map.prototype.forEach = () => { throw Error('mutable collection'); };
    Array.prototype.map = Array.prototype.filter = Array.prototype[Symbol.iterator] = () => { throw Error('mutable array'); };
    String.prototype.trim = String.prototype.toLowerCase = String.prototype.split = String.prototype.slice = String.prototype.indexOf = () => { throw Error('mutable string'); };
    RegExp.prototype.exec = RegExp.prototype.test = () => { throw Error('mutable regexp'); };
    globalThis.Number = globalThis.parseFloat = () => { throw Error('mutable number'); };
    const names = ['children','style','layout','rank','priority','color'];
    for (let i = 0; i < names.length; i++) {
      Object.defineProperty(Object.prototype, names[i], { configurable:true, set() { leaks++; } });
    }
    root.style.opacity = '0.5'; root.style.fontWeight = '700';
    root.style.cssText = 'opacity:0.5;font-weight:700';
    let rejected=false;try { root.style.opacity='2'; } catch { rejected=true; }
    if (!rejected || root.style.opacity !== '0.5' || leaks) throw Error('validator or private state');
  },
  update() {
    if (leaks) throw Error('private collect leaked');
    sheet.replaceSync('*{box-sizing:border-box}:root{background:blue}');
    rootNode.style.opacity='1';
  },
  dispose() { if (leaks) throw Error('private state leaked'); },
});
