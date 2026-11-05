// 通用四边框：同一 child 读取 native 客户盒并更新/恢复，不复制股票业务。
let box, text, report, stage = 0;
const initial = '2px solid red';
const restore = () => { box.style.border = initial; box.style.borderLeft = '8px solid blue'; box.style.borderBottom = '6px solid green'; };
const read = () => {
  const b = box.getBoundingClientRect(), t = text.getBoundingClientRect();
  report.textContent = JSON.stringify([b.width, b.height, box.clientWidth, box.clientHeight, t.x, t.y]);
};
NeuroSurface.define({
  mount({root}) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync('*{box-sizing:border-box}:root{display:flex;flex-direction:column;align-items:start;row-gap:0;background:black;color:white}.box{width:100px;height:64px;padding:3px}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<div class="box"><span>Border</span></div><p></p>';
    box = root.firstChild; text = box.firstChild; report = root.lastChild;
    report.style.display = 'none'; restore(); read();
  },
  update() {
    stage++;
    if (stage === 1) box.style.borderLeftWidth = '4px';
    else if (stage === 2) box.style.border = '0';
    else restore();
    read();
  },
});
