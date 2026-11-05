// 拒绝不支持/非法文档操作时必须保留原树、identity 和 live 集合。
NeuroSurface.define({ mount({ root }) {
  root.innerHTML = '<span id="keep">original</span><button disabled title="meta">button</button>';
  const original = root.innerHTML, first = root.firstChild, childNodes = root.childNodes;
  const reject = operation => {
    let failed = false; try { operation(); } catch { failed = true; }
    if (!failed || root.firstChild !== first || root.innerHTML !== original || childNodes.length !== 2) throw Error("not atomic");
  };
  for (const html of [
    '<script>globalThis.pwned=true</script>', '<input type="password">', '<video></video>',
    '<span style="color:red">css</span>', '<button onclick="evil()">event</button>',
    '<span data-x="a" DATA-X="b"></span>', '<span id=unquoted></span>',
    '<span></div>', '<span>unclosed', '<span/>', '<span>&#xD800;</span>',
    '<div>'.repeat(32) + '</div>'.repeat(32), 'x'.repeat(65537),
  ]) reject(() => root.innerHTML = html);
  reject(() => root.replaceChildren(root));
  reject(() => first.appendChild(root));
  reject(() => root.replaceChildren(first, root));
  reject(() => root.appendChild({}));
  reject(() => new (class extends Node { constructor() { super("script"); } })());
  reject(() => first.classList.add("valid", "bad token"));
  reject(() => first.setAttribute("style", "color:red"));
  reject(() => root.querySelectorAll("div span"));
  reject(() => root.querySelectorAll('[data-x^="prefix"]'));
  const fragment = document.createDocumentFragment();
  reject(() => fragment.appendChild(fragment));
  if (globalThis.pwned !== undefined || typeof fetch !== "undefined" || typeof CSSStyleSheet !== "function") throw Error("host boundary");
} });
