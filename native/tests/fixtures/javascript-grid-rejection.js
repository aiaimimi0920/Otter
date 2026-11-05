// 合法声明替换先全量验证；任何轨道/端点/shorthand 失败不能改变旧 Map 或 sheet。
NeuroSurface.define({ mount({ root }) {
  const sheet = new CSSStyleSheet(); sheet.replaceSync(':root{display:grid;grid-template-columns:40px auto;gap:2px}');
  document.adoptedStyleSheets = [sheet]; root.textContent = 'atomic';
  root.style.cssText = 'row-gap:8px!important;column-gap:4px;grid-row:2/4';
  const saved = root.style.cssText;
  const reject = operation => { let failed=false; try { operation(); } catch { failed=true; } if (!failed) throw Error('not rejected'); };
  const invalid = [
    'grid-template-columns:repeat(65,1fr)', 'grid-template-columns:repeat(2,repeat(2,1fr))',
    'grid-template-columns:repeat(auto-fit,1fr)', 'grid-template-columns:subgrid',
    'grid-template-columns:minmax(1fr,2fr)', 'grid-template-columns:minmax(0,fit-content(1px))',
    'grid-template-columns:[named] 1fr', 'grid-template-columns:0fr',
    'grid-template-columns:8193px', 'grid-template-columns:1001fr',
    'grid-auto-columns:1fr 2fr', 'grid-auto-rows:none', 'grid-auto-flow:row dense',
    'grid-column:0', 'grid-column:span 0', 'grid-column:2049/2050',
    'grid-column:span 2/span 3', 'grid-row:2/3/4', 'grid-row:1/',
    'gap:1px 2px 3px', 'gap:1px 2%', 'gap:normal', 'place-items:baseline',
    'place-self:center baseline', 'place-content:baseline', 'justify-items:baseline', 'align-items:start end',
    'grid-template-columns:1fr' + String.fromCharCode(0xa0) + '1fr',
    'grid-template-columns:' + '0'.repeat(300) + '1px',
    'grid-template-columns:repeat(64,' + '0'.repeat(64) + '1px)',
    'grid-template-columns:0.' + '0'.repeat(99) + '1fr',
  ];
  for (const declaration of invalid) {
    reject(() => root.style.cssText = 'color:red;' + declaration);
    reject(() => sheet.replaceSync(':root{' + declaration + '}'));
    if (root.style.cssText !== saved) throw Error('partial replacement');
  }
  reject(() => root.style.setProperty('gap','3px 4px','urgent'));
  reject(() => root.style.setProperty('gap','3px nope'));
  if (root.style.cssText !== saved) throw Error('partial shorthand');
  // 根不能是 Grid item；保留上面的原子性证据后移除合法但上下文不适用的声明。
  root.style.removeProperty('grid-row');
} });
