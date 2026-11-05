// 每个模式独立 operation：整体style/sheet转换，以及主动变更后的抛错/解析拒绝。
NeuroSurface.define({ mount({ root, snapshot }) {
  const mode = snapshot.state.quote, sheetMode = mode % 2 === 1;
  const failure = Math.floor(mode / 2), sentinel = {};
  let sheet, conversions = 0, caught = false, returned = false;
  root.style.boxSizing = "border-box";
  if (sheetMode) {
    sheet = new CSSStyleSheet();
    sheet.replaceSync(":root{width:40px;color:blue}");
    document.adoptedStyleSheets = [sheet];
  } else root.style.cssText = "box-sizing:border-box;width:40px;color:blue";
  if (root.clientWidth !== 40) throw Error("initial variant geometry");
  const value = { toString() {
    conversions++;
    if (root.clientWidth !== 40) throw Error("preconversion variant geometry");
    if (sheetMode) sheet.replaceSync(":root{width:50px;color:blue}");
    else root.style.width = "50px";
    if (root.clientWidth !== 50) throw Error("reentrant variant mutation");
    if (failure === 1) throw sentinel;
    const body = "width:60px;color:green" + (failure === 2 ? ";unknown:1" : "");
    returned = true;
    return sheetMode ? ":root{" + body + "}" : "box-sizing:border-box;" + body;
  } };
  try {
    if (sheetMode) sheet.replaceSync(value);
    else root.style = value;
  } catch (error) {
    caught = true;
    if (failure === 0 || (failure === 1 && error !== sentinel)) throw error;
    if (failure === 2 && (!returned || error?.message !== "Unsupported or invalid native DOM operation")) throw error;
  }
  if (conversions !== 1 || caught !== (failure !== 0)) throw Error("variant conversion result");
  if (root.clientWidth !== (failure === 0 ? 60 : 50)) throw Error("variant commit geometry");
  root.textContent = "ready";
} });
