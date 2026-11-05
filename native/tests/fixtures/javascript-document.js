// 非业务专用文档夹具：真实树、查询、反射、移动、委托和同闭包 update。
const check = (ok, stage) => { if (!ok) throw new Error(`document fixture: ${stage}`); };
let label, button, count = 0;
NeuroSurface.define({
  mount({ root, emit }) {
    root.innerHTML = '<span id="status" data-ref="status">A&amp;&lt;&#x1F642;&#20013;</span>' +
      '<section id="first"><button type="button" class="base" data-action="send"><strong>Go</strong></button></section>' +
      '<article id="second"></article>';
    label = root.querySelector('[data-ref="status"]');
    button = root.querySelector("button");
    const first = document.getElementById("first"), second = document.getElementById("second");
    check(label.textContent === "A&<🙂中", "entities");
    check(document.querySelector("#surface-root") === root && !root.querySelector("#surface-root"), "document root");
    check(button instanceof Node && button instanceof Element && button instanceof HTMLElement &&
      button instanceof HTMLButtonElement && label.firstChild instanceof Text, "prototypes");
    check(button.querySelector("strong").closest("[data-action]") === button && button.matches(".base"), "closest");
    const live = first.children, captured = root.querySelectorAll("[data-action]");
    button.dataset.periodValue = "daily"; button.dataset.__proto__ = "metadata";
    check(button.getAttribute("data-period-value") === "daily" && Object.getPrototypeOf(button.dataset) === null, "dataset");
    check(Object.fromEntries(Object.entries(button.dataset)).periodValue === "daily", "dataset enumeration");
    button.setAttribute("data-period-value", "hourly");
    check(button.dataset.periodValue === "hourly", "attribute to dataset");
    delete button.dataset.periodValue;
    check(!button.hasAttribute("data-period-value"), "dataset delete");
    button.classList.add("active", "base"); button.classList.toggle("active", false);
    button.classList.toggle("next", true);
    check(button.className === "base next" && button.getAttribute("class") === "base next", "class reflection");
    button.className = "fresh fresh"; button.classList.remove("fresh");
    check(button.classList.length === 0 && button.className === "", "class overwrite");
    button.disabled = true; check(button.hasAttribute("disabled"), "disabled set");
    button.removeAttribute("disabled"); check(!button.disabled, "disabled remove");
    const fragment = document.createDocumentFragment();
    fragment.append("lead", button);
    check(live.length === 0 && fragment.firstChild instanceof Text && fragment.firstElementChild === button, "move out");
    check(captured.length === 1 && captured.item(0) === button, "static query identity");
    second.replaceChildren(fragment);
    check(fragment.childNodes.length === 0 && second.children.length === 1 && button.parentElement === second, "fragment transfer");
    check(second.removeChild(second.firstChild).data === "lead" && second.firstChild === button, "remove text");
    first.appendChild(button);
    check(live.length === 1 && live[0] === button && [...live][0] === button && second.lastChild === null, "live move in");
    second.append("tail"); second.textContent = "";
    check(root.innerHTML.includes('<strong>Go</strong>'), "serialization");
    label.textContent = "ready";
    root.addEventListener("click", event => {
      check(event.target.closest("[data-action]") === button && event.currentTarget === root, "delegated identity");
      count++; label.textContent = `clicked:${count}`;
      emit({ nodeId: "refresh", event: "click", action: "refresh", class: "discrete", payload: { count, source: button.dataset.action } });
    });
  },
  update({ snapshot }) { label.textContent = `updated:${snapshot.state.quote}:${count}`; button.firstElementChild.textContent = "Again"; },
});
