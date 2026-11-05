// 通用 fragment/查询/属性委托真正接入 Application listener/FIFO/update；不是股票适配。
let label, current, count = 0;
NeuroSurface.define({
  async mount({ root, snapshot, emit }) {
    current = snapshot;
    root.innerHTML = '<span data-ref="label"></span><button type="button" data-action="refresh"><strong>刷新</strong></button>';
    label = root.querySelector('[data-ref="label"]');
    label.textContent = `原生🙂:${current.state.quote}:0`;
    root.addEventListener("click", async event => {
      const button = event.target.closest("[data-action]");
      if (!(button instanceof HTMLButtonElement) || button.dataset.action !== "refresh") throw new Error("invalid delegated target");
      await Promise.resolve();
      count++;
      for (const phase of [1, 2]) {
        await Promise.resolve();
        emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete",
          payload: { count, phase, quote: NeuroSurface.snapshot().state.quote } });
      }
      label.textContent = `原生🙂:${current.state.quote}:${count}`;
    });
  },
  async update({ snapshot }) {
    if (snapshot.state.fail) throw new Error("fixture update rejected");
    current = snapshot;
    await Promise.resolve();
    label.dataset.revision = String(snapshot.revision);
    label.classList.toggle("updated", true);
    if (!label.hasAttribute("data-revision") || !label.classList.contains("updated")) throw new Error("attribute reflection failed");
    label.textContent = `原生🙂:${current.state.quote}:${count}`;
  },
});
