// 通用 wrapper/Canvas 悬停；rAF 才更新 tooltip 和像素，不翻译真实股票业务。
NeuroSurface.define({ mount({ root }) {
  const status = document.createElement("p"), wrap = document.createElement("div");
  const a = document.createElement("canvas"), b = document.createElement("canvas"), field = document.createElement("input");
  root.style.cssText = "display:flex;flex-direction:column;gap:4px";
  status.style.cssText = "height:24px;box-sizing:border-box";
  wrap.style.cssText = "display:flex;flex-direction:row;gap:8px";
  a.width = b.width = 64; a.height = b.height = 40;
  a.style.cssText = b.style.cssText = "width:64px;height:40px;box-sizing:border-box";
  status.textContent = "ready"; field.value = "focus";
  root.append(status, wrap, field); wrap.append(a, b);
  const log = [], label = n => n === a ? "a" : n === b ? "b" : n === wrap ? "w" : n === null ? "null" : "other";
  let pending = false, hovered = false;
  const paint = () => { pending = false; status.textContent = log.join(">");
    const ctx = a.getContext("2d"); ctx.fillStyle = hovered ? "#ff0000" : "#0000ff"; ctx.fillRect(0, 0, 64, 40); };
  for (const n of [a, b, wrap]) for (const type of ["pointerover", "pointerout", "pointerenter", "pointerleave", "pointermove"]) {
    n.addEventListener(type, e => {
      if (!(e instanceof PointerEvent) || !(e instanceof MouseEvent) || !e.isTrusted || e.button !== -1 || e.buttons !== 0 ||
          e.pointerId !== 1 || e.pointerType !== "mouse" || !e.isPrimary || e.bubbles !== (type !== "pointerenter" && type !== "pointerleave") ||
          e.cancelable !== (type !== "pointerenter" && type !== "pointerleave") || e.composed !== e.bubbles) throw Error("hover fields");
      log.push(type + ":" + label(n) + ":" + label(e.relatedTarget));
      if (type === "pointermove") { hovered = e.target === a; const r = a.getBoundingClientRect();
        if (!Number.isFinite(e.clientX - r.left) || !Number.isFinite(e.clientY - r.top)) throw Error("hover geometry"); }
      if (type === "pointerleave" && n === wrap) hovered = false;
      if (!pending) { pending = true; requestAnimationFrame(paint); }
    });
  }
} });
