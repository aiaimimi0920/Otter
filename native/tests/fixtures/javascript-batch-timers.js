// 通用 timer→两事件→收敛消费者；复用原 TLS/FIFO，不新增远端动作或网络能力。
let timerRoot, timerLabel, timerButton, quote = 0, count = 0, ticks = 0, settled = 0;
function initialTimer() {
  if (timerRoot.getBoundingClientRect().width <= 0) throw Error("timer viewport");
  timerButton.style.backgroundColor = ++ticks === 1 ? "#00ff00" : "#0000ff";
  if (ticks < 2) setTimeout(initialTimer, 0);
}
NeuroSurface.define({
  mount({ root, snapshot, emit }) {
    timerRoot = root; quote = snapshot.state.quote;
    root.innerHTML = '<p></p><button>刷新</button>';
    timerLabel = root.firstChild; timerButton = root.lastChild;
    timerLabel.textContent = `原生🙂:${quote}:${count}`;
    setTimeout(initialTimer, 0);
    timerButton.addEventListener("click", () => {
      if (ticks !== 2 || settled !== count) throw Error("previous timer not settled");
      count++;
      setTimeout(() => {
        timerLabel.textContent = `原生🙂:${quote}:${count}`;
        for (let phase = 1; phase <= 2; phase++) {
          if (!emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete", payload: { count, phase, quote } })) throw Error("emit rejected");
        }
        setTimeout(() => { settled++; }, 0);
      }, 0);
    });
  },
  update({ snapshot }) {
    if (settled !== count - 1) throw Error("timer crossed pending owner");
    quote = snapshot.state.quote; timerLabel.textContent = `原生🙂:${quote}:${count}`;
  },
});
