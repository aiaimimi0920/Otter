// Actual ES-module fixture; do not rewrite JS as JSON Scene, and let native adapters consume DOM operations.
const ready = await Promise.resolve("原生🙂");
let label, button, current, count = 0, cleaned = false;
const assert = value => { if (!value) throw new Error("fixture invariant"); };
NeuroSurface.define({
  async mount({ root, snapshot, resources, emit }) {
    assert(ready === "原生🙂");
    assert(typeof fetch === "undefined" && typeof process === "undefined" && typeof require === "undefined");
    assert(typeof WebSocket === "undefined" && typeof setImmediate === "undefined");
    assert(typeof setTimeout === "function" && typeof setInterval === "function" &&
      typeof clearTimeout === "function" && typeof clearInterval === "function");
    assert(NeuroSurface.resource("logo") === resources.logo);
    assert(NeuroSurface.resource("toString") === undefined);
    current = snapshot;
    label = document.createElement("span"); label.textContent = `${ready}:${current.state.quote}:0`;
    button = document.createElement("button"); button.id = "refresh"; button.textContent = "刷新";
    button.addEventListener("click", async () => {
      await Promise.resolve(); await Promise.resolve();
      count++;
      label.textContent = `${ready}:${current.state.quote}:${count}`;
      assert(emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete",
        payload: { count, quote: NeuroSurface.snapshot().state.quote, label: ready },
        generation: 9999, baseRevision: 9999, attachmentId: "forged" }));
    });
    root.replaceChildren(label, button);
    await Promise.resolve();
    return () => { cleaned = true; };
  },
  async update({ snapshot }) {
    current = snapshot; label.textContent = `${ready}:${current.state.quote}:${count}`;
    // The API returns copies; scripts cannot rewrite authoritative state or event contracts for the next emit.
    const copy = NeuroSurface.snapshot(); copy.revision = 9999;
    Promise.resolve().then(async () => {
      await Promise.resolve(); await Promise.resolve();
      label.textContent = `${ready}:${current.state.quote}:${count}`;
    });
    await Promise.resolve();
  },
  suspend() { label.textContent = "paused"; },
  resume() { label.textContent = `${ready}:${current.state.quote}:${count}`; },
  dispose() { assert(cleaned); },
});
