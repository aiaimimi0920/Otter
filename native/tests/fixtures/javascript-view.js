// 主干同 revision 视图更新：真实 child 保留 DOM、closure 和资源 owner。
let label, button, state, resourceOwner, count = 0, updates = 0, mounts = 0;
const assert = value => { if (!value) throw new Error("view fixture invariant"); };
const render = () => {
  const copy = NeuroSurface.snapshot();
  assert(copy.viewId === state.viewId && copy.revision === state.revision);
  label.textContent = `${state.viewId ?? "none"}:${count}:${updates}:${state.revision}:${state.state.quote}:${mounts}`;
};
NeuroSurface.define({
  mount({ root, snapshot, resources }) {
    state = snapshot; resourceOwner = resources; mounts++;
    label = document.createElement("span");
    button = document.createElement("button"); button.id = "refresh";
    button.textContent = "increment";
    button.addEventListener("click", () => {
      count++; render();
      if (state.state.emit) assert(NeuroSurface.emit({ nodeId: "refresh", event: "click", action: "refresh",
        class: "discrete", payload: { count, viewId: NeuroSurface.snapshot().viewId } }));
    });
    root.replaceChildren(label, button); render();
  },
  async update({ snapshot, resources }) {
    assert(resources === resourceOwner);
    state = snapshot; updates++;
    // 外部副本的改动不能污染宿主私有 snapshot。
    const copy = NeuroSurface.snapshot(); copy.viewId = "forged"; copy.revision = 9999;
    await Promise.resolve(); render();
  },
});
