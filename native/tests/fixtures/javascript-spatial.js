// 通用定位/裁剪消费者；child、真实 HWND、封存产品共用，不包含股票业务分支。
let spatialRoot, spatialStage, spatialFill, spatialOverlay, spatialFlow, spatialReport;
const spatialEncode = JSON.stringify;
function spatialMeasure() {
  spatialReport.textContent = spatialEncode({ stage: spatialStage.getBoundingClientRect(),
    fill: spatialFill.getBoundingClientRect(), overlay: spatialOverlay.getBoundingClientRect(),
    flow: spatialFlow.getBoundingClientRect(), client: [spatialStage.clientWidth, spatialStage.clientHeight] });
}
NeuroSurface.define({
  mount({ root }) {
    spatialRoot = root;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`*{box-sizing:border-box}:root{position:relative;height:100px;background:#101010}
      .stage{position:relative;width:100px;height:60px;min-height:60px;padding:4px;border-width:2px;border-style:solid;border-color:#00ff00;background:#0000ff;overflow:hidden}
      .flow{width:20px;height:10px;min-height:10px;background:white}
      .fill{position:absolute;inset:0;background:#0000ff}
      button{position:absolute;left:70px;top:10px;width:50px;height:30px;border-radius:0;background:#ff0000;padding:0}
      .report{display:none}`);
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<div class="stage"><div class="flow"></div><div class="fill"></div><button></button></div><p class="report"></p>';
    spatialStage = root.firstChild; spatialReport = root.lastChild;
    [spatialFlow, spatialFill, spatialOverlay] = spatialStage.children;
    spatialMeasure();
    spatialOverlay.addEventListener("click", () => {
      spatialStage.style.overflow = "visible";
      spatialOverlay.style.left = "80px";
      spatialOverlay.style.backgroundColor = "#ffff00";
      spatialMeasure();
    });
  },
  update() {
    spatialStage.style.overflow = "hidden";
    spatialStage.style.width = "140px";
    spatialOverlay.style.inset = "auto 5px 5px auto";
    spatialOverlay.style.backgroundColor = "#00ffff";
    spatialMeasure();
  },
  dispose() { spatialRoot.replaceChildren(); },
});
