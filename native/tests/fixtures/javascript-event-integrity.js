// 污染公开方法/intrinsic 不触达私有登记；returnValue 仍遵守 cancelable/passive。
let saved;
NeuroSurface.define({ mount({ root }) {
  saved = root;
  const dispatch = EventTarget.prototype.dispatchEvent, apply = Reflect.apply;
  const poison = () => { throw Error('public event intrinsic'); };
  Array.prototype.map = Array.prototype.filter = Array.prototype.push = Array.prototype[Symbol.iterator] = poison;
  WeakMap.prototype.get = WeakMap.prototype.set = WeakMap.prototype.has = poison;
  Map.prototype.get = Map.prototype.set = Map.prototype.forEach = poison;
  EventTarget.prototype.dispatchEvent = Event.prototype.preventDefault = poison;
  Promise.prototype.then = poison;
  Object.defineProperty(Promise.prototype, 'constructor', { get: poison });
  Object.defineProperty(Promise, Symbol.species, { get: poison });
  const target = new EventTarget(); let count = 0;
  const event = new Event('cancel', { cancelable: true });
  target.addEventListener('cancel', value => {
    value.returnValue = false;
    if (value.defaultPrevented) throw Error('passive cancellation');
    count++;
  }, { passive: true, once: true });
  if (!apply(dispatch, target, [event]) || count !== 1) throw Error('passive dispatch');
  target.addEventListener('cancel', value => { value.returnValue = false; count++; }, { once: true });
  if (apply(dispatch, target, [event]) || !event.defaultPrevented || count !== 2) throw Error('private cancellation');
  event.returnValue = true;
  if (event.returnValue) throw Error('uncanceled event');
  const fixed = new Event('fixed'); fixed.returnValue = false;
  if (fixed.defaultPrevented) throw Error('uncancelable event');
  root.textContent = 'event-integrity-ready';
  const button = document.createElement('button'); root.appendChild(button);
  button.addEventListener('click', event => {
    if (!event.isTrusted) throw Error('untrusted native click');
    event.returnValue = false;
    if (!event.defaultPrevented) throw Error('native cancellation');
    root.firstChild.textContent = 'event-integrity-clicked';
  });
}, update() { saved.textContent = 'event-integrity-updated'; } });
