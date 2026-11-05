// 同一真实 child/HWND/Application 消费；焦点方法不自动产生远端 action。
let field, status, count = 0;
NeuroSurface.define({
  mount({ root }) {
    root.innerHTML = '<p>ready</p><input value="seed"><button>focus</button><button>blur</button>';
    status = root.children[0]; field = root.children[1];
    // 断开/disabled 调用不排请求；借用方法仍须验证原生 DOM identity。
    const detached = document.createElement('input'); detached.focus(); detached.blur();
    detached.disabled = true; root.appendChild(detached); detached.focus(); detached.blur(); detached.remove();
    let rejected = false; try { field.focus({preventScroll:true}); } catch { rejected = true; }
    if (!rejected) throw Error('unsupported focus options');
    rejected = false; try { HTMLInputElement.prototype.focus.call({}); } catch { rejected = true; }
    if (!rejected) throw Error('focus receiver');
    root.children[2].addEventListener('click', () => { field.focus(); status.textContent = 'focus-requested'; });
    root.children[3].addEventListener('click', () => { field.blur(); status.textContent = 'blur-requested'; });
    field.addEventListener('input', event => {
      if (!event.isTrusted) throw Error('untrusted input');
      status.textContent = `input|${field.value}`;
      field.blur();
    });
    field.addEventListener('change', event => {
      if (!event.isTrusted) throw Error('untrusted change');
      status.textContent = `change|${field.value}`; count++;
      for (let phase = 1; phase <= 2; phase++) {
        if (!NeuroSurface.emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{count,phase,quote:NeuroSurface.snapshot().state.quote}})) throw Error('emit');
      }
    });
  },
  update({ snapshot }) { field.value = `server:${snapshot.state.quote}`; },
});
