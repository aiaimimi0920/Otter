// 通用字体合同，不包含真实插件业务；声明失败不提交，主动转换 mutation 仍保留。
let root, sheet;
NeuroSurface.define({
  mount(context) {
    root = context.root;
    sheet = new CSSStyleSheet();
    sheet.replaceSync(':root{font-family:MissingOtterFamily,Consolas,monospace;font-size:16px;color:white} .override{font-family:"Segoe UI",sans-serif}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<span>WWii 0123</span><span class="override">WWii 0123</span>';
    const before = root.style.cssText;
    let byteRejected = false;
    try { root.style.fontFamily = '\u00a0'.repeat(255) + 'a'; }
    catch (error) { if (error.message !== 'Unsupported or invalid native DOM operation') throw error; byteRejected = true; }
    if (!byteRejected || root.style.cssText !== before) throw Error('font raw UTF-8 admission');
    for (const value of ['', 'url(x)', 'inherit', 'cursive', 'ui-serif', 'a,', '"a,b"', '"abc', 'a'.repeat(32), Array(9).fill('a').join(','), Array(8).fill('a'.repeat(30)).join(','), Array(3).fill('一'.repeat(31)).join(',')]) {
      if (!value) continue; // 空值是 CSS removeProperty，不是非法替换。
      let rejected = false;
      try { root.style.cssText = 'color:red;font-family:' + value; }
      catch (error) { if (error.message !== 'Unsupported or invalid native DOM operation') throw error; rejected = true; }
      if (!rejected || root.style.cssText !== before) throw Error('font atomicity');
    }
    root.style.fontFamily = ' Microsoft   YaHei , "微软雅黑", sans-serif';
    if (root.style.fontFamily !== '"Microsoft YaHei","微软雅黑",sans-serif') throw Error('font reflection');
    root.style.fontFamily = ['a'.repeat(30), ...Array(7).fill('a'.repeat(29))].join(',');
    if (root.style.fontFamily.length !== 256) throw Error('font canonical boundary');
    root.style.removeProperty('font-family');
  },
  update() {
    root.style.fontFamily = 'Segoe UI, sans-serif';
    root.lastChild.style.fontFamily = 'MissingOtterFamily, monospace';
  },
});
