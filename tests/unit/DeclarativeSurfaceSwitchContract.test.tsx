// @vitest-environment jsdom
// Windows 原生迁移的旧 JSX 合同对照；不以 jsdom 结果替代真实浏览器验收。
import { afterEach, expect, it } from "vitest";
import { render } from "solid-js/web";
import { DeclarativeSurface } from "../../src/features/surfaces/components/DeclarativeSurface";
import { SURFACE_PROTOCOL_VERSION, type SurfaceEvent, type SurfaceSnapshot } from "../../src/features/surfaces/services/surfaceProtocol";

afterEach(() => { document.body.innerHTML = ""; });

it.each(["input", "span", "label"])("records Switch %s activation and ancestor event order", (target) => {
    const events: SurfaceEvent[] = [];
    const snapshot: SurfaceSnapshot = {
        protocolVersion: SURFACE_PROTOCOL_VERSION,
        instanceId: "instance:test", attachmentId: "attachment:test", artId: "test/switch",
        artVersion: "1.0.0", revision: 1,
        scene: { id: "root", type: "column", events: { click: "outer" }, children: [
            { id: "switch", type: "switch", props: { value: false }, events: { change: "toggle" } },
        ] },
    };
    const host = document.createElement("div");
    document.body.append(host);
    const dispose = render(() => <DeclarativeSurface unitId="unit:test" snapshot={snapshot} generation={1} onEvent={(event) => { events.push(event); }} />, host);
    try {
        const element = host.querySelector(target === "label" ? ".surface-switch" : `.surface-switch ${target}`) as HTMLElement;
        element.click();
        expect((host.querySelector("input") as HTMLInputElement).checked).toBe(true);
        expect(events.map(event => [event.nodeId, event.event, event.action])).toEqual([
            ...Array.from({ length: target === "input" ? 1 : 2 }, () => ["root", "click", "outer"]),
            ["switch", "change", "toggle"],
        ]);
        expect(events.filter(event => event.event === "change")).toHaveLength(1);
        expect(events.at(-1)?.payload).toEqual({ value: true });
    } finally { dispose(); }
});
