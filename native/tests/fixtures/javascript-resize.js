// 无业务替身：观察真实盒，observer 只排 rAF，rAF 才消费新 bounds 并改变可见颜色。
let host, target, label, observer, count = 0, frames = 0, entry, bounds = 0;
const report = () => { label.textContent = JSON.stringify({ count, frames, entry, bounds }); };
NeuroSurface.define({
  mount({ root }) {
    host = root;
    root.innerHTML = '<div></div><p>waiting</p>';
    target = root.firstElementChild; label = root.lastElementChild;
    target.style.cssText = 'box-sizing:border-box;width:50%;height:80px;min-height:80px;padding:4px;border-width:2px;border-style:solid;border-color:#000000;background:#ff0000';
    observer = new ResizeObserver(function(entries, owner) {
      if (owner !== observer || this !== observer || entries.length !== 1 || entries[0].target !== target) throw Error('identity');
      for (const value of entries) entry = {
        width: value.contentRect.width, height: value.contentRect.height,
        left: value.contentRect.left, top: value.contentRect.top,
        border: value.borderBoxSize[0].inlineSize, content: value.contentBoxSize[0].inlineSize,
        device: value.devicePixelContentBoxSize[0].inlineSize,
      };
      count++; report();
      requestAnimationFrame(() => {
        frames++; bounds = target.getBoundingClientRect().width;
        target.style.backgroundColor = frames % 2 ? '#00ff00' : '#0000ff'; report();
      });
    });
    observer.observe(target);
    if (count !== 0) throw Error('synchronous observe');
  },
  update({ snapshot }) {
    const mode = snapshot.state.quote;
    if (mode === 1) target.style.display = 'none';
    else if (mode === 2) target.style.removeProperty('display');
    else if (mode === 3) target.remove();
    else if (mode === 4) host.replaceChildren(target, label);
    else if (mode === 5) { observer.disconnect(); observer.disconnect(); }
  },
  dispose() { observer.disconnect(); },
});
