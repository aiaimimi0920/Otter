// 与既有 FIFO 夹具同协议，额外把每次真实同步客户区/DPI 读取呈现在文字中。
let label, host, spatialButton, current, count = 0;
function paintGeometryLabel() {
  spatialButton.style.left = (8 + count * 4) + "px";
  const rect = spatialButton.getBoundingClientRect();
  if (rect.left !== 8 + count * 4 || rect.top !== 32 || rect.width !== 120 || rect.height !== 30) throw Error("positioned application bounds");
  label.textContent = `原生🙂:${current.state.quote}:${count}|${host.clientWidth}|${devicePixelRatio}`;
}
NeuroSurface.define({
  async mount({ root, snapshot, emit }) {
    host = root; current = snapshot;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync('*{box-sizing:border-box}:root{position:relative;height:100px;overflow:hidden}button{position:absolute;left:8px;top:32px;width:120px;height:30px}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<span></span><button type="button" data-action="refresh"><strong>刷新</strong></button>';
    label = root.firstChild; spatialButton = root.lastChild; paintGeometryLabel();
    root.addEventListener("click", async event => {
      const button = event.target.closest("[data-action]");
      if (!(button instanceof HTMLButtonElement)) throw Error("invalid target");
      await Promise.resolve(); count++;
      for (const phase of [1, 2]) {
        await Promise.resolve();
        emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete",
          payload: { count, phase, quote: NeuroSurface.snapshot().state.quote } });
      }
      paintGeometryLabel();
    });
  },
  async update({ snapshot }) {
    current = snapshot; await Promise.resolve(); paintGeometryLabel();
  },
});
