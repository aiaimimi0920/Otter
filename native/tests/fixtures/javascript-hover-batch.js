// ancestor listener 处理无按键 move/leave；沿原测试 Grant 发两项动作，业务不固定在 Rust。
let status, quote, count = 0, hovering = false;
NeuroSurface.define({
  mount({ root, snapshot, emit }) {
    quote = snapshot.state.quote;
    status = document.createElement("p"); const wrap = document.createElement("div"), canvas = document.createElement("canvas");
    canvas.width = 64; canvas.height = 40; canvas.style.cssText = "width:64px;height:40px;box-sizing:border-box";
    root.append(status, wrap); wrap.append(canvas);
    const context = canvas.getContext("2d"), paint = color => { context.fillStyle = color; context.fillRect(0, 0, 64, 40); };
    wrap.addEventListener("pointermove", event => {
      if (!event.isTrusted || event.target !== canvas || event.currentTarget !== wrap || event.button !== -1 || event.buttons !== 0) throw Error("hover move");
      hovering = true; paint("#ff0000"); status.textContent = "hover";
    });
    wrap.addEventListener("pointerleave", event => {
      if (!hovering || !event.isTrusted || event.target !== wrap || event.bubbles || event.relatedTarget !== null) throw Error("hover leave");
      hovering = false; count++; paint("#0000ff");
      for (const phase of [1, 2]) if (!emit({ nodeId: "refresh", action: "refresh", event: "click", class: "discrete", payload: { count, phase, quote } })) throw Error("emit");
      status.textContent = `原生🙂:${quote}:${count}`;
    });
    status.textContent = `原生🙂:${quote}:${count}`;
  },
  update({ snapshot }) { quote = snapshot.state.quote; status.textContent = `原生🙂:${quote}:${count}`; }
});
