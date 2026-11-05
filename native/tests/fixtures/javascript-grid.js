// 通用 Grid/cascade/动态显隐负载；不含业务模板、网络宿主或第二套布局算法。
let first, second, third, host;
NeuroSurface.define({
  mount({ root, emit }) {
    host = root;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`
      * { box-sizing:border-box; }
      :root { display:grid; height:80px; grid-template-columns:40px minmax(0,1fr);
        grid-template-rows:repeat(2,30px); gap:4px 8px; align-content:start;
        background:#101010; color:white; font-size:12px; text-align:left; }
      button { min-width:0; min-height:0; padding:0; border-width:0; border-radius:0;
        background:#00ff00; }
      button:first-child { grid-column:1; grid-row:1; }
      .footer { grid-column:1/-1; grid-row:2; }
      :root.wide { grid-template-columns:60px 1fr; row-gap:6px; }
      :root.updated { grid-template-columns:80px 1fr; grid-template-rows:24px 40px; gap:8px 12px; }
      :root.updated .footer { grid-column:2; }
    `);
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<button>one</button><button>two</button><button class="footer">three</button>';
    first = root.children[0]; second = root.children[1]; third = root.children[2];
    first.addEventListener('click', () => {
      host.className = 'wide'; first.style.background = 'blue'; second.style.display = 'none';
      if (!emit({ nodeId:'refresh', event:'click', action:'refresh', class:'discrete', payload:{grid:true} })) throw Error('emit');
    });
    return () => { document.adoptedStyleSheets = []; };
  },
  update() {
    host.className = 'updated'; second.style.display = ''; first.style.background = 'yellow';
  },
});
