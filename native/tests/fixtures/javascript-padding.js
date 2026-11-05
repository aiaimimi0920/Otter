// 四边声明、cascade、反射/移除与原子拒绝；同份夹具由模型、child 和产品消费。
let saved, button;
const rejectPadding = operation => {
  try { operation(); } catch (error) {
    if (!(error instanceof Error) || error.message !== 'Unsupported or invalid native DOM operation') throw error;
    return;
  }
  throw Error('padding operation accepted');
};
NeuroSurface.define({mount({root}) {
  saved = root;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(':root{box-sizing:border-box;padding:3px 7px 11px 9px!important;padding-left:13px!important;padding:9px} button{padding:1px 3px 5px 7px}');
  document.adoptedStyleSheets = [sheet];
  root.style.cssText = 'display:flex;flex-direction:row;gap:4px;width:200px;height:80px;border-width:2px;border-style:solid;border-color:blue;background:#101010';
  root.innerHTML = '<button>Run</button><div></div>'; button = root.firstChild;
  button.style.cssText = 'box-sizing:border-box;width:32px;height:40px;border-width:0;background:#00ff00;color:white';
  root.lastChild.style.cssText = 'box-sizing:border-box;width:20px;height:10px;background:#00ffff';
  const probe = document.createElement('div');
  for (const value of ['2px','2px 4px','2px 4px 6px','2px 4px 6px 8px']) {
    probe.style.padding = value; if (probe.style.padding !== value) throw Error('padding reflection');
  }
  probe.style.setProperty('padding','2px','important');
  probe.style.paddingLeft = '3px';
  if (probe.style.padding !== '') throw Error('padding mixed priority');
  probe.style.padding = '2px 4px';
  if (probe.style.getPropertyPriority('padding') !== '') throw Error('padding explicit priority reset');
  if (probe.style.removeProperty('padding') !== '2px 4px' || probe.style.length) throw Error('padding removal');
  probe.style.padding = '.5px 2.px 0 3px';
  if (probe.style.padding !== '0.5px 2px 0 3px') throw Error('padding decimal');
  const previous = probe.style.cssText;
  for (const value of ['1px invalid','1px 2px 3px 4px 5px','1%','-1px','auto','1em','calc(1px + 2px)','8193px','1px\u00a02px','1'.repeat(257)+'px']) {
    rejectPadding(() => probe.style.padding = value);
    if (probe.style.cssText !== previous) throw Error('partial padding replacement');
    rejectPadding(() => sheet.replaceSync(':root{padding:' + value + '}'));
  }
  rejectPadding(() => probe.style.padding = ' '.repeat(256)+'0');
  const marker = {}; let thrown = false;
  try { probe.style.padding = {toString(){throw marker}}; } catch (error) { if(error !== marker) throw error; thrown = true; }
  if (!thrown || probe.style.cssText !== previous) throw Error('padding conversion replacement');
}, update() {
  saved.style.setProperty('padding','7px 11px 13px 17px','important');
  button.style.paddingLeft = '9px';
}});
