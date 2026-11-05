// 通用长标题：真实 CSS、同步 native 几何及同 child 更新，不复制任何业务应用。
let title, report, stage = 0;
const text = '长标题 e\u0301 😀 Native text with a hidden suffix';
const read = () => {
  const rect = title.getBoundingClientRect();
  report.textContent = JSON.stringify([rect.width, rect.height, title.textContent, title.style.textOverflow]);
};
NeuroSurface.define({
  mount({root}) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync('*{box-sizing:border-box}:root{display:flex;flex-direction:column;align-items:start;row-gap:0;color:white;background:black}.title{white-space:nowrap;text-align:left;font:24px Segoe UI;overflow:hidden;width:84px;height:32px;text-overflow:ellipsis}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<span class="title"></span><p></p>';
    title = root.firstChild; report = root.lastChild; report.style.display = 'none';
    title.textContent = text; read();
  },
  update() {
    stage++;
    if (stage === 1) title.style.textOverflow = 'clip';
    else { title.style.textOverflow = 'ellipsis'; title.style.width = stage === 2 ? '360px' : '84px'; }
    read();
  },
});
