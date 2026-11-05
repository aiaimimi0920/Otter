// 通用同步布局消费者；同一文档用于 child、真实 HWND 和封存产品，不包含业务专例。
const encodeGeometry = JSON.stringify;
let geometryRoot, geometryButton, geometryAuto, geometryOutput, geometryHidden;
function geometryReport() {
  const first = geometryButton.getBoundingClientRect();
  const second = geometryAuto.getBoundingClientRect();
  const root = geometryRoot.getBoundingClientRect();
  const hidden = geometryHidden.firstChild.getBoundingClientRect();
  const saved = first.width;
  try { first.width = 12345; } catch {}
  if (first.width !== saved || first.right !== first.left + first.width) throw Error("mutable rect");
  geometryOutput.textContent = encodeGeometry({ first, second, root, hidden,
    client: [geometryButton.clientWidth, geometryButton.clientHeight],
    offset: [geometryButton.offsetWidth, geometryButton.offsetHeight], ratio: devicePixelRatio });
}
NeuroSurface.define({
  mount({ root }) {
    geometryRoot = root;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`*{box-sizing:border-box}:root{display:grid;grid-template-columns:40px minmax(0,1fr);grid-template-rows:60px 60px;gap:4px 8px;height:140px;place-content:start}
      button{border-width:2px;border-style:solid;border-color:#00ff00;border-radius:0;padding:3px;height:60px;background:#0000ff;color:white}
      p{grid-column:1/-1;height:60px;font-size:10px}`);
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<button>measure</button><div></div><p></p><div><button>hidden</button></div>';
    [geometryButton, geometryAuto, geometryOutput, geometryHidden] = root.children;
    geometryHidden.style.display = "none";
    const detached = document.createElement("div");
    if (detached.offsetWidth !== 0 || detached.getBoundingClientRect().bottom !== 0) throw Error("detached");
    const inline = document.createElement("span"); geometryButton.append(inline);
    let rejected = false;
    try { inline.getBoundingClientRect(); } catch { rejected = true; }
    if (!rejected) throw Error("flattened inline"); inline.remove();
    geometryReport();
    geometryButton.addEventListener("click", () => {
      const before = geometryButton.getBoundingClientRect();
      geometryRoot.style.gridTemplateColumns = "70px minmax(0,1fr)";
      const after = geometryButton.getBoundingClientRect();
      if (before.width === after.width || before.width !== 40 || after.width !== 70) throw Error("stale box");
      geometryReport();
    });
  },
  update() { geometryRoot.style.gridTemplateColumns = "90px minmax(0,1fr)"; geometryReport(); },
  resume() { geometryReport(); },
  dispose() { geometryRoot.replaceChildren(); },
});
