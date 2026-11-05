// 同一通用两事件 FIFO 夹具，增加真实 native click 的 capture/target/bubble 与清理断言。
let label, current, count = 0, once = 0, last, trace;
const check = (value, message) => { if (!value) throw Error(message); };
NeuroSurface.define({ mount({ root, snapshot, emit }) {
  current = snapshot;
  root.innerHTML = '<span></span><div><button type="button" data-action="refresh"><strong>刷新</strong></button></div>';
  label = root.firstChild;
  const parent = root.lastChild, button = parent.firstChild;
  label.textContent = `原生🙂:${current.state.quote}:0`;
  const removed = () => { throw Error('removed capture listener'); };
  root.addEventListener('click', function(event) {
    trace = ['root-c'];
    check(this === root && event.currentTarget === root && event.eventPhase === 1, 'root capture');
    check(event.target === button && event.isTrusted && event.bubbles && event.cancelable && event.composed, 'native event');
    check(Number.isFinite(event.timeStamp) && event.timeStamp >= 0, 'clock');
    const path = event.composedPath();
    check(path.length === 3 && path[0] === button && path[1] === parent && path[2] === root, 'fixed native path');
    path.length = 0;
    check(event.composedPath().length === 3, 'path copy');
    root.removeEventListener('click', removed, true);
  }, true);
  root.addEventListener('click', removed, true);
  parent.addEventListener('click', event => { check(event.eventPhase === 1, 'parent capture'); trace.push('parent-c'); }, true);
  button.addEventListener('click', event => { check(event.eventPhase === 2, 'target capture'); trace.push('target-c'); }, true);
  button.addEventListener('click', () => { once++; }, { once: true });
  button.addEventListener('click', function(event) {
    check(this === button && event.currentTarget === button && event.eventPhase === 2, 'target bubble');
    last = event; trace.push('target-b');
  });
  parent.addEventListener('click', event => { check(event.eventPhase === 3, 'parent bubble'); trace.push('parent-b'); });
  root.addEventListener('click', async event => {
    trace.push('root-b');
    check(trace.join(',') === 'root-c,parent-c,target-c,target-b,parent-b,root-b' && once === 1, 'phase or once');
    event.returnValue = false;
    check(event.defaultPrevented && !event.returnValue, 'native cancellation');
    await Promise.resolve();
    check(event.currentTarget === null && event.eventPhase === 0 && event.composedPath().length === 0, 'async cleanup');
    count++;
    for (const phase of [1, 2]) {
      await Promise.resolve();
      check(emit({ nodeId: 'refresh', event: 'click', action: 'refresh', class: 'discrete',
        payload: { count, phase, quote: NeuroSurface.snapshot().state.quote } }), 'authorized emit');
    }
    label.textContent = `原生🙂:${current.state.quote}:${count}`;
  });
}, update({ snapshot }) {
  current = snapshot;
  check(last && last.currentTarget === null && last.eventPhase === 0 && once === 1, 'same child update');
  label.textContent = `原生🙂:${current.state.quote}:${count}`;
} });
