let field, status, quote, count = 0, attempts = 0, trace = [];
const record = text => { if (trace.length >= 32) throw Error('trace capacity'); trace.push(text); status.textContent = trace.join('>'); };
NeuroSurface.define({
  mount({ root, snapshot, emit }) {
    quote = snapshot.state.quote;
    status = document.createElement('p');
    field = document.createElement('input'); field.value = 'initial';
    root.append(status, field);
    field.addEventListener('input', event => {
      if (!event.isTrusted || event.cancelable) throw Error('input contract');
      if (attempts === 0) trace = [];
      record(`input:${field.value}`);
    });
    field.addEventListener('keydown', event => {
      if (!(event instanceof KeyboardEvent) || !(event instanceof Event) || !event.isTrusted || !event.cancelable || !event.bubbles || !event.composed || event.key !== 'Enter' || event.isComposing) throw Error('keyboard contract');
      if (++attempts === 1) event.preventDefault();
      record(`down:${event.defaultPrevented ? 'cancel' : 'allow'}:${event.repeat ? 'repeat' : 'first'}`);
    });
    field.addEventListener('keyup', event => {
      if (!(event instanceof KeyboardEvent) || !event.isTrusted || event.repeat || event.key !== 'Enter') throw Error('keyup contract');
      record('up');
    });
    field.addEventListener('change', event => {
      if (!event.isTrusted || event.cancelable) throw Error('change contract');
      record(`change:${field.value}`); ++count;
      for (const phase of [1, 2]) if (!emit({ nodeId: 'refresh', event: 'click', action: 'refresh', class: 'discrete', payload: { count, phase, quote } })) throw Error('emit contract');
    });
  },
  update({ snapshot }) {
    quote = snapshot.state.quote; field.value = `server:${quote}`;
    attempts = 0; trace = []; status.textContent = `server:${quote}:${count}`;
  },
});
