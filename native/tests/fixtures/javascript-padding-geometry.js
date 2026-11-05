// 写入使同 operation 几何缓存失效；ResizeObserver 的内容起点/尺寸按真实四边求值。
let saved, child, report, updated = false;
const configure = () => {
  saved.style.padding = updated ? '7px 11px 13px 17px' : '3px 7px 11px 13px';
  const rect = child.getBoundingClientRect();
  if (rect.x !== (updated ? 19 : 15) || rect.y !== (updated ? 9 : 5)) throw Error('padding geometry');
};
NeuroSurface.define({mount({root}) {
  saved = root;
  root.style.cssText = 'box-sizing:border-box;display:flex;flex-direction:row;gap:0;width:200px;height:80px;border-width:2px;border-style:solid;padding:1px 2px 3px 4px';
  root.innerHTML = '<div></div><span></span>'; child = root.firstChild; report = root.lastChild;
  child.style.cssText = 'box-sizing:border-box;width:20px;height:10px';
  if (child.getBoundingClientRect().x !== 6) throw Error('initial padding geometry');
  const marker = {}; let caught = false;
  try { root.style.padding = {toString(){
    root.style.paddingLeft = '9px';
    if (child.getBoundingClientRect().x !== 11) throw Error('active padding conversion geometry');
    throw marker;
  }}; } catch(error) { if(error !== marker) throw error; caught = true; }
  if (!caught || child.getBoundingClientRect().x !== 11) throw Error('padding reentrant mutation lost');
  configure();
  new ResizeObserver(entries => {
    const entry = entries[0], rect = entry.contentRect, ratio = devicePixelRatio;
    const width = updated ? 168 : 176, height = updated ? 56 : 62;
    if (rect.x !== (updated ? 17 : 13) || rect.y !== (updated ? 7 : 3) || rect.width !== width || rect.height !== height) throw Error('padding observer content');
    if (entry.borderBoxSize[0].inlineSize !== 200 || saved.clientWidth !== 196 || saved.clientHeight !== 76) throw Error('padding client box');
    const device = entry.devicePixelContentBoxSize[0];
    const x = rect.x + 2, y = rect.y + 2;
    if (device.inlineSize !== Math.round((x + width) * ratio) - Math.round(x * ratio) || device.blockSize !== Math.round((y + height) * ratio) - Math.round(y * ratio)) throw Error('padding device content');
    report.textContent = JSON.stringify([rect.x,rect.y,rect.width,rect.height,device.inlineSize,device.blockSize]);
  }).observe(root);
}, update() { updated = true; configure(); }});
