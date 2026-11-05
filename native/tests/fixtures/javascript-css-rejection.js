// 原子 sheet/adoption/inline 拒绝，保留同 identity 的原声明与真实文档。
NeuroSurface.define({ mount({ root }) {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync('*{box-sizing:border-box} :root{color:green} span{color:red}');
  document.adoptedStyleSheets = [sheet]; root.innerHTML = '<span>safe</span>';
  const leaf = root.firstChild; leaf.style.color = 'blue';
  const reject = operation => { let failed = false; try { operation(); } catch { failed = true; } if (!failed) throw Error('not rejected'); };
  for (const source of [
    '@import "https://invalid.test/a";', '@media(max-width:500px){span{color:red}}',
    'span{display:subgrid}', 'span{background:url(x)}', 'span{color:var(--x)}',
    'span{padding:1px 2px 3px 4px 5px}', 'span{font-family:cursive}', 'span{width:8193px}',
    'span:hovering{color:red}', 'span:nth-child(2n of .cell){color:red}', 'span{color:red;unknown:1}',
    'span{color:red', 'span{color:red}/*',
    'span{color:rgb(9999999999999999999999999999999999999999,0,0)}',
    'span{font-weight:400.5}',
    'span{color:rgb(' + String.fromCharCode(0x2000) + '0,0,0)}',
  ]) {
    reject(() => sheet.replaceSync(source));
  }
  reject(() => document.adoptedStyleSheets = [sheet, {}]);
  const copy = document.adoptedStyleSheets; copy.length = 0;
  if (document.adoptedStyleSheets.length !== 1 || document.adoptedStyleSheets[0] !== sheet) throw Error('adoption leaked');
  reject(() => leaf.style.cssText = 'color:red;grid-template-columns:subgrid');
  reject(() => leaf.style.color = 'url(x)'); reject(() => leaf.style.fontFamily = 'cursive');
  if (leaf.style.color !== 'blue' || leaf.style.length !== 1 || leaf.style.item(0) !== 'color') throw Error('inline changed');
  leaf.style.setProperty('color','white','important'); leaf.style.setProperty('color','blue');
  if (leaf.style.getPropertyPriority('color') !== '') throw Error('inline important not replaced');
} });
