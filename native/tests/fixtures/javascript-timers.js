// 同 child/HWND/产品共享的通用 timer 消费者；不含远端请求或业务逻辑。
let timerBox, timerReport, timerCount = 0, repeating;
function timerCallback(token, amount) {
  if (this !== globalThis || token !== timerBox || amount !== 1) throw Error("timer arguments");
  timerCount += amount;
  timerBox.style.width = `${timerCount * 10}%`;
  timerBox.style.backgroundColor = timerCount === 1 ? "#00ff00" : "#0000ff";
  timerReport.textContent = JSON.stringify({ count: timerCount, width: timerBox.getBoundingClientRect().width, time: performance.now() });
  if (timerCount === 2) clearTimeout(repeating);
}
NeuroSurface.define({
  mount({ root }) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync("*{box-sizing:border-box}:root{position:relative;height:100px;background:#101010}.box{position:absolute;left:0;top:0;width:10px;height:20px;background:#ff0000}.report{display:none}");
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<div class="box"></div><p class="report">0</p>';
    timerBox = root.firstChild; timerReport = root.lastChild;
    repeating = setInterval(timerCallback, 0, timerBox, 1);
    clearInterval(setTimeout(() => { throw Error("cancelled timeout ran"); }, 0));
    return () => clearTimeout(repeating);
  },
  update() { setTimeout(() => timerReport.textContent = JSON.stringify({ count: timerCount, quote: NeuroSurface.snapshot().state.quote }), 0); },
});
