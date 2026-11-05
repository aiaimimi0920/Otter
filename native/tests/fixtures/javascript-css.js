// 通用 author cascade/inline 更新负载，不包含股票业务或宿主旁路。
let first, second, base, accent;
NeuroSurface.define({
  mount({ root, emit }) {
    base = new CSSStyleSheet(); accent = new CSSStyleSheet();
    base.replaceSync(`
      * { box-sizing: border-box; }
      :root { display:flex; flex-direction:row; gap:8px; padding:4px;
        border-width:2px; border-style:solid; border-color:#0000ff; background:#101010;
        color:#ffffff; font-size:12px; text-align:left; }
      button { width:40px; height:40px; min-width:0; min-height:0; padding:0;
        border-width:0; border-radius:0; background:red; }
      .tile { background:#00ff00 !important; }
    `);
    accent.replaceSync(':root > button.tile[data-view=hot] { width:50px; background:#0000ff; }');
    document.adoptedStyleSheets = [base, accent];
    root.innerHTML = '<button class="tile" data-view="hot">one</button><button class="tile">two</button>';
    first = root.firstElementChild; second = root.lastElementChild;
    first.style.width = '32px'; first.style.background = 'red';
    first.addEventListener('click', () => {
      first.style.setProperty('background', '#0000ff', 'important');
      first.style.width = '48px'; second.style.display = 'none';
      if (!emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{css:true}})) throw Error('emit');
    });
    return () => { document.adoptedStyleSheets = document.adoptedStyleSheets.filter(sheet => sheet !== accent && sheet !== base); };
  },
  update() {
    first.style.background = ''; first.style.width = '';
    first.classList.remove('tile'); first.dataset.view = 'cold';
    second.style.display = ''; second.classList.remove('tile'); second.style.flexGrow = '1';
    accent.replaceSync('button[data-view=cold] { width:60px; background:#ffff00; }');
  },
});
