// 每个目标/阶段快照、移除标记、固定路径、停止与嵌套分派。
NeuroSurface.define({ mount({ root }) {
  const check = (value, message) => { if (!value) throw Error(message); };
  root.innerHTML = '<div><button>mutation</button></div>';
  const parent = root.firstChild, button = parent.firstChild, log = [];
  const removed = () => log.push('removed'), late = () => log.push('late');
  button.addEventListener('edit', () => { log.push('first'); button.removeEventListener('edit', removed); button.addEventListener('edit', late); });
  button.addEventListener('edit', removed);
  button.dispatchEvent(new Event('edit'));
  check(log.join(',') === 'first', 'removed or added callback invoked in same phase');
  log.length = 0; button.dispatchEvent(new Event('edit'));
  check(log.join(',') === 'first,late', 'later dispatch');
  const duplicate = () => log.push('duplicate');
  button.addEventListener('dual', duplicate, true); button.addEventListener('dual', duplicate);
  button.addEventListener('dual', duplicate, { once: true });
  button.removeEventListener('dual', duplicate, true); log.length = 0;
  button.dispatchEvent(new Event('dual')); button.dispatchEvent(new Event('dual'));
  check(log.join(',') === 'duplicate,duplicate', 'identity includes capture and ignores once');
  root.addEventListener('move', () => { log.push('root-c'); parent.remove(); }, true);
  parent.addEventListener('move', () => log.push('parent-b'));
  root.addEventListener('move', () => log.push('root-b'));
  log.length = 0; button.dispatchEvent(new Event('move', { bubbles: true }));
  check(log.join(',') === 'root-c,parent-b,root-b', 'dispatch path changed after removal');
  root.appendChild(parent);
  button.addEventListener('stop', event => { log.push('one'); event.stopPropagation(); });
  button.addEventListener('stop', () => log.push('two')); parent.addEventListener('stop', () => log.push('parent'));
  log.length = 0; button.dispatchEvent(new Event('stop', { bubbles: true }));
  check(log.join(',') === 'one,two', 'stopPropagation stopped same-target callback');
  button.addEventListener('immediate', event => { log.push('one'); event.stopImmediatePropagation(); });
  button.addEventListener('immediate', () => log.push('two'));
  log.length = 0; button.dispatchEvent(new Event('immediate'));
  check(log.join(',') === 'one', 'immediate propagation');
  const target = new EventTarget(); let count = 0;
  target.addEventListener('nested', () => { count++; target.dispatchEvent(new Event('nested')); }, { once: true });
  target.dispatchEvent(new Event('nested')); check(count === 1, 'once removed too late');
  const event = new Event('reuse'), marker = {};
  target.addEventListener('reuse', current => {
    let rejected = false; try { target.dispatchEvent(current); } catch { rejected = true; }
    check(rejected, 'active redispatch'); throw marker;
  }, { once: true });
  let error; try { target.dispatchEvent(event); } catch (value) { error = value; }
  check(error === marker && event.currentTarget === null && event.eventPhase === 0, 'throw cleanup');
  check(target.dispatchEvent(event), 'redispatch after throw');
  root.textContent = 'event-mutation-ready';
} });
