// 同一 child 中 family mutation 立即失效原投影；真实 DirectWrite 贡献 max-content 宽度。
let text, report;
const read = () => {
  const before = text.getBoundingClientRect().width;
  text.style.fontFamily = 'Segoe UI';
  const changed = text.getBoundingClientRect().width;
  text.style.removeProperty('font-family');
  const restored = text.getBoundingClientRect().width;
  if (before !== restored || before === changed) throw Error('font geometry invalidation');
  report.textContent = JSON.stringify([before, changed, restored]);
};
NeuroSurface.define({
  mount({root}) {
    root.style.cssText = 'display:flex;flex-direction:column;align-items:start;font-size:16px;font-family:MissingOtterFamily,monospace';
    root.innerHTML = '<span>WWii 0123</span><span></span>';
    text = root.firstChild; report = root.lastChild; read();
  },
  update() { read(); },
});
