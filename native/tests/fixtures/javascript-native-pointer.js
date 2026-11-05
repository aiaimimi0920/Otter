NeuroSurface.define({
  mount({ root }) {
    const status = document.createElement("p"), canvas = document.createElement("canvas");
    status.style.cssText = "height:24px;box-sizing:border-box";
    canvas.width = 64; canvas.height = 64;
    canvas.style.cssText = "width:64px;height:64px;box-sizing:border-box";
    root.append(status, canvas);
    const context = canvas.getContext("2d"); let log = "";
    for (const phase of ["pointerdown", "pointermove", "pointerup", "pointercancel"]) {
      canvas.addEventListener(phase, event => {
        if (!(event instanceof PointerEvent) || !(event instanceof MouseEvent) || !(event instanceof Event) ||
          !event.isTrusted || event.target !== canvas || event.currentTarget !== canvas || event.eventPhase !== 2 ||
          event.pointerId !== 1 || event.pointerType !== "mouse" || !event.isPrimary || !event.bubbles || !event.composed) throw Error("pointer identity");
        const terminal = phase === "pointerup" || phase === "pointercancel";
        if (event.buttons !== (terminal ? 0 : 1) || event.button !== (phase === "pointermove" || phase === "pointercancel" ? -1 : 0)) throw Error("pointer buttons");
        event.preventDefault();
        if (event.defaultPrevented !== (phase !== "pointercancel")) throw Error("pointer cancel state");
        log += (log ? ">" : "") + phase;
        status.textContent = log;
        context.fillStyle = phase === "pointerdown" ? "#ff0000" : phase === "pointermove" ? "#00ff00" : "#0000ff";
        context.fillRect(0, 0, 64, 64);
      });
    }
    canvas.addEventListener("click", () => { log += ">click"; status.textContent = log; });
  }
});
