// Unitless 继承保留乘数；每次节点字号变化都由原 native 测量 owner 重新解析。
let text, report;
const read = () => {
  const before = text.getBoundingClientRect().height;
  text.style.fontSize = '20px';
  const resized = text.getBoundingClientRect().height;
  text.style.lineHeight = 'normal';
  const normal = text.getBoundingClientRect().height;
  text.style.removeProperty('line-height');
  const inherited = text.getBoundingClientRect().height;
  text.style.removeProperty('font-size');
  const restored = text.getBoundingClientRect().height;
  if (before !== restored || resized !== inherited || before === resized || normal === resized) throw Error('typography geometry');
  let converted = false;
  text.style.font = {toString() {
    converted = true;
    if (text.getBoundingClientRect().height !== restored) throw Error('font conversion geometry');
    return '400 16px/1.5 Segoe UI';
  }};
  if (!converted || text.getBoundingClientRect().height !== before) throw Error('font conversion result');
  text.style.removeProperty('font');
  const marker = {};
  try { text.style.font = {toString() {
    text.style.fontSize = '20px';
    if (text.getBoundingClientRect().height !== resized) throw Error('active conversion mutation');
    throw marker;
  }}; } catch (error) { if (error !== marker) throw error; }
  if (text.getBoundingClientRect().height !== resized) throw Error('conversion mutation lost');
  text.style.removeProperty('font-size');
  report.textContent = JSON.stringify([before, resized, normal, inherited, restored]);
};
NeuroSurface.define({
  mount({root}) {
    root.style.cssText = 'display:flex;flex-direction:column;align-items:start;font:400 16px/1.5 Segoe UI;white-space:pre';
    root.innerHTML = '<span>First\nSecond</span><span></span>';
    text = root.firstChild; report = root.lastChild; read();
  },
  update() { read(); },
});
