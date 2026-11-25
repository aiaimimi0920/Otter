// At most one explicit result per operation; grants no permissions and does not write to the workspace, and the parent still revalidates independently.
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
      // Bound the UTF-16 lower estimate first; the parent also checks UTF-8, port counts, and every field, with no partial result publication on failure.
      if (typeof raw !== "string" || raw.length > 64 * 1024) fail();
      const copy = json.decode(raw);
      if (!copy.outputs || typeof copy.outputs !== "object" || isArray(copy.outputs)) fail();
      const names = keys(copy.outputs);
      for (let i = 0; i < names.length; i++) {
        const name = names[i];
        const output = copy.outputs[name];
        if (output && output.kind === "canvas") output.nodeId = canvasIdentity(output.nodeId);
      }
      // getter/toJSON may reenter commit; recheck after serialization rather than overwrite an already queued commit.
      if (!active || pending !== null) fail();
      pending = copy;
      return true;
    },
    end() { active = false; },
    collect() { return pending; },
  };
})
