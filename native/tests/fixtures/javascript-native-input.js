// 通用原生输入夹具；真实 EDIT、child 和产品门共用，不复制股票业务。
let field, status, count = 0;
NeuroSurface.define({
  mount({ root }) {
    root.innerHTML = '<p>ready</p><input type="text" value="seed" placeholder="代码"><button>inspect</button>';
    field = root.children[1]; status = root.children[0];
    if (!(field instanceof HTMLInputElement) || field.value !== 'seed' || field.defaultValue !== 'seed') throw Error('input identity');
    field.defaultValue = 'default';
    if (field.value !== 'default') throw Error('clean value reflection');
    field.value = 'line\r\nbreak';
    if (field.value !== 'linebreak') throw Error('single-line sanitization');
    for (const invalid of ['\0', '\uD800', 'x'.repeat(48 * 1024 + 1), '\n'.repeat(65537)]) {
      let rejected = false; try { field.value = invalid; } catch { rejected = true; }
      if (!rejected || field.value !== 'linebreak') throw Error('value atomic admission');
    }
    let rejected = false; try { field.type = 'password'; } catch { rejected = true; }
    if (!rejected || field.type !== 'text') throw Error('unsupported input type');
    rejected = false; try { field.appendChild(document.createTextNode('child')); } catch { rejected = true; }
    if (!rejected || field.childNodes.length) throw Error('input children');
    field.value = '初始🙂';
    field.defaultValue = 'seed';
    if (field.value !== '初始🙂' || field.getAttribute('value') !== 'seed' || !root.innerHTML.includes('<input type="text" value="seed" placeholder="代码">')) throw Error('dirty value reflection');
    const check = event => {
      if (!event.isTrusted || event.target !== field || !event.bubbles || event.cancelable) throw Error('native event');
      event.preventDefault();
      if (event.defaultPrevented || event.composed !== (event.type === 'input')) throw Error('post-edit event flags');
      status.textContent = `${event.type}|${field.value}|${++count}`;
      // 没有声明式事件的 DOM 字段也必须进入本地 listener，不能自动成为远端 action。
      if (event.type === 'change') for (let phase = 1; phase <= 2; phase++) {
        if (!NeuroSurface.emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{count:count / 2,phase,quote:NeuroSurface.snapshot().state.quote}})) throw Error('emit');
      }
    };
    field.addEventListener('input', check); field.addEventListener('change', check);
    root.children[2].addEventListener('click', () => { status.textContent = `click|${field.value}|${count}`; });
  },
  update({ snapshot }) { field.value = `server:${snapshot.state.quote}`; },
});
