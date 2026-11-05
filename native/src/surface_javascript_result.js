// 每 operation 至多一个显式结果；不赋予权限、不写工作区，父侧仍独立复核。
((json, fail, canvasIdentity) => {
  "use strict";
  let active = false, pending = null;
  const stringify = JSON.stringify;
  const keys = Object.keys;
  const isArray = Array.isArray;
  return {
    begin(enabled) { active = enabled; pending = null; },
    commit(outputs) {
      if (!active || pending !== null) fail();
      const raw = stringify({ protocolVersion: "otter.art.result.v1", outputs });
      // UTF-16 下界先限；父侧还检查 UTF-8、端口数和所有字段，失败不发布半份结果。
      if (typeof raw !== "string" || raw.length > 64 * 1024) fail();
      const copy = json.decode(raw);
      if (!copy.outputs || typeof copy.outputs !== "object" || isArray(copy.outputs)) fail();
      const names = keys(copy.outputs);
      for (let i = 0; i < names.length; i++) {
        const name = names[i];
        const output = copy.outputs[name];
        if (output && output.kind === "canvas") output.nodeId = canvasIdentity(output.nodeId);
      }
      // getter/toJSON 可以重入 commit；序列化后再次核查，不覆盖已排队提交。
      if (!active || pending !== null) fail();
      pending = copy;
      return true;
    },
    end() { active = false; },
    collect() { return pending; },
  };
})
