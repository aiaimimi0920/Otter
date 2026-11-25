// Generic dynamic-style fixture: only trusted native hover changes state, with reads through original DOM/geometry owners.
let status, left, right, a, b;
NeuroSurface.define({
  mount({root}) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`
      * { box-sizing:border-box }
      :root { width:200px; color:#000000 }
      :root:hover { color:#ffffff }
      .group { display:flex; width:80px; height:40px; background:#0000ff }
      .group:hover { background:#00ff00 }
      .cell:hover { width:30px; background:#ff0000 }
      .cell { width:20px; height:20px; background:#0000ff }
      .group:hover > .cell { height:24px }
      p { display:none }
    `);
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<p></p><div class="group"><div class="cell"></div><div class="cell"></div></div><div class="group"></div>';
    status = root.children[0]; left = root.children[1]; right = root.children[2];
    a = left.children[0]; b = left.children[1];
    a.addEventListener('pointerover', event => {
      // Verify synthetic notifications and native geometry separately; do not nest another layout stack around synchronous synthetic dispatch.
      if (!event.isTrusted) { status.textContent = 'synthetic'; return; }
      const box = a.getBoundingClientRect();
      status.textContent = 'enter|' + box.width + '|' + box.height;
    });
    // Synthetic events may notify listeners only, never fabricate the host's physical hover state.
    a.dispatchEvent(new PointerEvent('pointerover', {bubbles:true}));
    if (a.getBoundingClientRect().width !== 20) throw Error('synthetic hover changed CSS');
    let rejected = false;
    try { a.matches(':hover'); } catch { rejected = true; }
    if (!rejected) throw Error('DOM selector scope silently expanded');
  },
  update({snapshot}) {
    if (snapshot.state.quote === 45) right.append(a);
    else if (snapshot.state.quote === 46) a.remove();
    else if (snapshot.state.quote === 47) right.append(a);
    else if (snapshot.state.quote === 48) left.append(a);
  },
  suspend() {
    if (a.getBoundingClientRect().width !== 20) throw Error('suspend retained hover');
    status.textContent = 'suspended';
  },
  resume() {
    if (a.getBoundingClientRect().width !== 20) throw Error('resume replayed hover');
  },
});
