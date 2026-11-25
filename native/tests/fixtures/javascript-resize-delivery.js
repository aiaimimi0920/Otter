// 同批 unobserve 不撤回 active entry；disconnect 撤回后续批次，交付必须重新测量。
let parent, target, label, second, count = 0, log = [], latest;
const check = (value, message) => { if (!value) throw Error(message); };
NeuroSurface.define({
  mount({ root }) {
    root.innerHTML = '<div><div></div></div><p>waiting</p>';
    parent = root.firstElementChild; target = parent.firstElementChild; label = root.lastElementChild;
    parent.style.cssText = 'box-sizing:border-box;width:150px;height:80px';
    target.style.cssText = 'box-sizing:border-box;width:50px;height:30px;padding:4px;border-width:2px;border-style:solid';
    const first = new ResizeObserver(() => {
      log.push('first'); first.disconnect();
      Promise.resolve().then(() => {
        log.push('microtask');
        second.unobserve(target);
        third.disconnect();
        target.style.width = '75px';
      });
    });
    second = new ResizeObserver((entries, owner) => {
      check(owner === second && entries.length === 1, 'observer identity');
      const value = entries[0];
      check(value.target === target && Object.isFrozen(value) && Object.isFrozen(value.contentRect), 'entry identity');
      for (const key of ['contentBoxSize', 'borderBoxSize', 'devicePixelContentBoxSize']) {
        check(Object.isFrozen(value[key]) && Object.isFrozen(value[key][0]), 'box mutability');
      }
      count++; log.push('second');
      latest = {
        count, log: log.join(','), width: value.contentRect.width,
        border: value.borderBoxSize[0].inlineSize, device: value.devicePixelContentBoxSize[0].inlineSize,
      };
      label.textContent = JSON.stringify(latest);
    });
    const third = new ResizeObserver(() => { throw Error('disconnected batch delivered'); });
    first.observe(parent); second.observe(target); third.observe(target);
  },
  update({ snapshot }) {
    target.style.width = '100px';
    if (snapshot.state.quote === 2) second.observe(target);
  },
  dispose() { second.disconnect(); },
});
