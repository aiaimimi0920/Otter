// 与业务无关的字体 shorthand/cascade；拒绝整项后不得安装部分字号或 family。
let first, second;
const reject = operation => {
  try { operation(); } catch (error) {
    if (!(error instanceof Error) || error.message !== 'Unsupported or invalid native DOM operation') throw error;
    return;
  }
  throw Error('typography operation accepted');
};
NeuroSurface.define({
  mount({root}) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(':root{font:700 16px/1.5 Segoe UI,sans-serif} .large{font-size:20px} .reset{font:600 12px Consolas,monospace}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = '<span class="large">WWii 0123</span><button class="reset">WWii 0123</button>';
    first = root.firstChild; second = root.lastChild;
    second.style.font = '600 12px Consolas,monospace';
    if (second.style.font !== '600 12px/normal "Consolas",monospace') throw Error('font reflection');
    const previous = second.style.cssText;
    for (const value of ['italic 12px Arial', 'small-caps 12px Arial', '12pt Arial', '12px/.9 Arial', '12px/1001 Arial', '12px/20% Arial', '12px cursive', '12px Arial,', '12px/2', 'caption', '12px ' + '\u00a0'.repeat(255) + 'a']) {
      reject(() => second.style.font = value);
      if (second.style.cssText !== previous) throw Error('partial font replacement');
      reject(() => sheet.replaceSync(':root{font:700 16px/1.5 Segoe UI,sans-serif} span{font:' + value + '}'));
    }
    const marker = {};
    try { second.style.font = {toString(){throw marker}}; } catch (error) { if(error !== marker) throw error; }
    if (second.style.cssText !== previous) throw Error('conversion replacement');
    second.style.setProperty('font', 'bold 12px/1 Consolas', 'important');
    if (second.style.getPropertyPriority('font') !== 'important') throw Error('font priority');
    second.style.fontSize = '13px';
    if (second.style.font !== '') throw Error('mixed priority shorthand');
    second.style.font = '600 12px Consolas,monospace';
    if (second.style.getPropertyPriority('font') !== '') throw Error('font explicit reset');
    if (!second.style.removeProperty('font') || second.style.length) throw Error('font removal');
  },
  update() {
    first.parentElement.style.font = 'normal 18px/1.25 Segoe UI,sans-serif';
    first.style.fontSize = '24px'; second.style.font = 'bold 14px/2 Segoe UI,sans-serif';
  },
});
