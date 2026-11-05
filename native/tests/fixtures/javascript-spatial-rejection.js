// 每次错误都必须保留原四边/两轴；不能先写一半，再在最后一个值失败。
NeuroSurface.define({ mount({ root }) {
  root.style.cssText = "position:relative;inset:1px 2px 3px 4px;overflow:hidden";
  const reject = operation => { const old = root.style.cssText; let failed = false;
    try { operation(); } catch { failed = true; }
    if (!failed || root.style.cssText !== old) throw Error("non-atomic spatial rejection"); };
  for (const value of ["", "1px 2px 3px 4px 5px", "0 -1px", "1px var(--x)", "0 0 0 9000px", "1px 0 0 1001%", "\u00a01px", "1px".repeat(100)])
    if (value) reject(() => { root.style.inset = value; });
  for (const value of ["clip", "auto", "scroll", "hidden visible hidden", "visible bogus", "initial", "inherit"])
    reject(() => { root.style.overflow = value; });
  for (const value of ["fixed", "sticky", "inherit", "absolute relative"])
    reject(() => { root.style.position = value; });
  reject(() => { root.style.cssText = "position:relative;inset:2px 3px 4px 5px;overflow:scroll"; });
  const sheet = new CSSStyleSheet(); sheet.replaceSync(":root{position:relative;color:green}");
  let failed = false; try { sheet.replaceSync(":root{inset:0 0 0 -1px}"); } catch { failed = true; }
  if (!failed) throw Error("sheet replacement");
  document.adoptedStyleSheets = [sheet];
} });
