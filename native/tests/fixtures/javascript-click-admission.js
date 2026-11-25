// Use host Command::Click: verify admission and dispatch separately, with microtasks complete before result collection.
let host, parent, button, label, mode = 0, log = [];
const check = (value, message) => { if (!value) throw Error(message); };
NeuroSurface.define({
  mount({ root }) {
    host = root;
    root.innerHTML = '<p>ready</p><div><button>go</button></div>';
    label = root.firstElementChild; parent = root.lastElementChild; button = parent.firstElementChild;
    root.addEventListener('click', event => {
      check(event.isTrusted, 'native trust');
      if (event.target === root) {
        check(event.composedPath().length === 1, 'root path');
        log = ['root'];
      } else {
        log = ['capture'];
        check(event.eventPhase === Event.CAPTURING_PHASE, 'capture phase');
        if (mode === 1) parent.remove();
      }
    }, true);
    button.addEventListener('click', event => {
      const path = event.composedPath();
      check(event.eventPhase === Event.AT_TARGET && path.length === 3 &&
        path[0] === button && path[1] === parent && path[2] === root, 'fixed path');
      if (mode === 3) throw Error('native callback rejected');
      log.push('target');
    });
    parent.addEventListener('click', () => log.push('parent'));
    root.addEventListener('click', () => {
      log.push('bubble');
      Promise.resolve().then(() => {
        log.push('microtask'); label.textContent = log.join(',');
      });
    });
    Object.defineProperty(Node.prototype, 'parentNode', { get() { throw Error('public parent getter'); } });
  },
  update({ snapshot }) {
    mode = snapshot.state.quote;
    button.disabled = mode === 2;
    if (mode === 4) parent.remove();
    else host.appendChild(parent);
  },
});
