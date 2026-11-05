let status, canvas, context, quote, count = 0, drawing = false;
NeuroSurface.define({
  mount({ root, snapshot, emit }) {
    quote = snapshot.state.quote;
    status = document.createElement("p"); canvas = document.createElement("canvas");
    canvas.width = 64; canvas.height = 64;
    canvas.style.cssText = "width:64px;height:64px;box-sizing:border-box";
    root.append(status, canvas); context = canvas.getContext("2d");
    const paint = color => { context.fillStyle = color; context.fillRect(0, 0, 64, 64); };
    canvas.addEventListener("pointerdown", event => {
      if (!event.isTrusted || event.pointerId !== 1 || event.buttons !== 1) throw Error("down");
      drawing = true; paint("#ff0000"); status.textContent = "down";
    });
    canvas.addEventListener("pointermove", event => {
      if (!drawing || !event.isTrusted || event.button !== -1) throw Error("move");
      paint("#00ff00"); status.textContent = "move";
    });
    canvas.addEventListener("pointerup", event => {
      if (!drawing || !event.isTrusted || event.buttons !== 0) throw Error("up");
      drawing = false; count++; paint("#0000ff");
      for (const phase of [1, 2]) if (!emit({ nodeId: "refresh", action: "refresh", event: "click", class: "discrete", payload: { count, phase, quote } })) throw Error("emit");
      status.textContent = `原生🙂:${quote}:${count}`;
    });
    canvas.addEventListener("pointercancel", () => { drawing = false; });
    status.textContent = `原生🙂:${quote}:${count}`;
  },
  update({ snapshot }) { quote = snapshot.state.quote; status.textContent = `原生🙂:${quote}:${count}`; }
});
