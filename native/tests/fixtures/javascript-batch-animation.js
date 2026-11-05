// 通用 rAF emit 消费者；原两事件 FIFO/更新协议不变，不添加远端动作或网络旁路。
let animationRoot, animationLabel, animationButton, quote = 0, count = 0, frames = 0, settled = 0;
function initialFrame(t) {
  if (t > performance.now() || animationRoot.getBoundingClientRect().width <= 0) throw Error("frame viewport");
  animationButton.style.backgroundColor = ++frames === 1 ? "#00ff00" : "#0000ff";
  if (frames < 2) requestAnimationFrame(initialFrame);
}
NeuroSurface.define({
  mount({ root, snapshot, emit }) {
    animationRoot = root; quote = snapshot.state.quote;
    root.innerHTML = '<p></p><button>刷新</button>';
    animationLabel = root.firstChild; animationButton = root.lastChild;
    animationLabel.textContent = `原生🙂:${quote}:${count}`;
    requestAnimationFrame(initialFrame);
    animationButton.addEventListener("click", () => {
      if (frames !== 2 || settled !== count) throw Error("previous frame not settled");
      count++;
      requestAnimationFrame(() => {
        animationLabel.textContent = `原生🙂:${quote}:${count}`;
        for (let phase = 1; phase <= 2; phase++) {
          if (!emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete", payload: { count, phase, quote } })) throw Error("emit rejected");
        }
        requestAnimationFrame(() => { settled++; });
      });
    });
  },
  update({ snapshot }) {
    if (settled !== count - 1) throw Error("frame crossed pending owner");
    quote = snapshot.state.quote; animationLabel.textContent = `原生🙂:${quote}:${count}`;
  },
});
