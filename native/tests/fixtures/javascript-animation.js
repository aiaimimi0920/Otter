// The same frame consumer serves child/HWND/product; dimension reads still use actual native measurement, not manual business-state advancement.
let animatedBox, animationReport, animationTicks = 0, previousFrame = -1;
function animateFrame(timestamp) {
  if (this !== globalThis || !Number.isFinite(timestamp) || timestamp < previousFrame || timestamp > performance.now()) throw Error("frame clock");
  previousFrame = timestamp; animationTicks++;
  animatedBox.style.width = `${animationTicks * 10}%`;
  animatedBox.style.backgroundColor = animationTicks === 1 ? "#00ff00" : "#0000ff";
  animationReport.textContent = JSON.stringify({ ticks: animationTicks, width: animatedBox.getBoundingClientRect().width, timestamp });
  if (animationTicks < 2) requestAnimationFrame(animateFrame);
}
NeuroSurface.define({
  mount({ root }) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync("*{box-sizing:border-box}:root{position:relative;height:100px;background:#101010}.box{position:absolute;left:0;top:0;width:10px;height:20px;background:#ff0000}.report{display:none}");
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<div class="box"></div><p class="report">0</p>';
    animatedBox = root.firstChild; animationReport = root.lastChild;
    requestAnimationFrame(animateFrame);
    cancelAnimationFrame(requestAnimationFrame(() => { throw Error("cancelled frame ran"); }));
  },
  update() { requestAnimationFrame(animateFrame); },
});
