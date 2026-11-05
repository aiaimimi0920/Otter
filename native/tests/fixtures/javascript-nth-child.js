// 通用四列网格；文本不计兄弟位置，重排沿同一文档/child，不包含股票业务。
let owner, first, sheet;
NeuroSurface.define({
  mount({root}) {
    owner = root; sheet = new CSSStyleSheet();
    sheet.replaceSync(`
      *{box-sizing:border-box}
      :root{display:grid;grid-template-columns:repeat(4,30px);grid-template-rows:repeat(2,24px);width:120px;height:48px;gap:0;padding:0;background:#101010}
      button{width:30px;height:24px;min-width:0;min-height:0;padding:0;border-width:0;border-radius:0;background:#0000ff}
      :root > .cell:nth-child(4n){background:#00ff00}
    `);
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = ' \n' + '<button class="cell"></button> \n'.repeat(8);
    first = root.firstElementChild;
    first.addEventListener('click', () => root.appendChild(first));
    return () => { document.adoptedStyleSheets = []; };
  },
  update() {
    const others = Array.from(owner.children).filter(node => node !== first);
    owner.replaceChildren(first, ...others);
  },
});
