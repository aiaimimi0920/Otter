// 通用事件路径、取消与接收者；合成事件不冒充原生键盘/指针交付。
NeuroSurface.define({ mount({ root }) {
  const check = (value, message) => { if (!value) throw Error(message); };
  root.innerHTML = '<div><button>event</button></div>';
  const parent = root.firstChild, button = parent.firstChild, log = [];
  check(button instanceof EventTarget && new EventTarget() instanceof EventTarget, 'target class');
  const remember = name => function(event) {
    check(this === event.currentTarget && event.target === button && !event.isTrusted, 'receiver');
    check(event.composedPath()[0] === button && event.composedPath()[2] === root, 'path');
    log.push(name + event.eventPhase);
  };
  root.addEventListener('sample', remember('root-c'), true);
  parent.addEventListener('sample', remember('parent-c'), { capture: true });
  button.addEventListener('sample', remember('target-c'), true);
  button.addEventListener('sample', remember('target-b'));
  parent.addEventListener('sample', remember('parent-b'));
  root.addEventListener('sample', remember('root-b'));
  const event = new Event('sample', { bubbles: true, cancelable: true, composed: true });
  check(button.dispatchEvent(event), 'dispatch result');
  check(log.join(',') === 'root-c1,parent-c1,target-c2,target-b2,parent-b3,root-b3', 'phase order');
  check(event.currentTarget === null && event.eventPhase === Event.NONE && event.composedPath().length === 0, 'cleanup');
  check(event.target === button && event.bubbles && event.cancelable && event.composed, 'event properties');
  check(Event.CAPTURING_PHASE === 1 && Event.AT_TARGET === 2 && Event.BUBBLING_PHASE === 3, 'constants');
  log.length = 0; button.dispatchEvent(new Event('sample'));
  check(log.join(',') === 'root-c1,parent-c1,target-c2,target-b2', 'non-bubbling capture');
  const target = new EventTarget(), canceled = new Event('cancel', { cancelable: true });
  target.addEventListener('cancel', event => { event.preventDefault(); check(!event.defaultPrevented, 'passive'); }, { passive: true, once: true });
  check(target.dispatchEvent(canceled), 'passive result');
  const object = { count: 0, handleEvent(event) { check(this === object, 'object receiver'); this.count++; event.preventDefault(); } };
  target.addEventListener('cancel', object);
  check(!target.dispatchEvent(canceled) && canceled.defaultPrevented && !canceled.returnValue && object.count === 1, 'cancel result');
  target.removeEventListener('cancel', object);
  check(!target.dispatchEvent(canceled) && object.count === 1, 'persistent cancel flag');
  root.textContent = 'event-target-ready';
}, update() {
  const target = new EventTarget(); let count = 0;
  const callback = () => count++;
  target.addEventListener('change', callback, { once: true });
  target.dispatchEvent(new Event('change')); target.dispatchEvent(new Event('change'));
  if (count !== 1) throw Error('same child once');
} });
