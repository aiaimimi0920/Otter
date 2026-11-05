// 同一 operation 内读取、移位、移除、重插，再跨 update 读取；不维护影子几何。
let rootNode, a, b, c, output, values;
const widths = () => [a,b,c].map(node => node.getBoundingClientRect().width);
NeuroSurface.define({
  mount({root}) {
    rootNode = root;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync('*{box-sizing:border-box}:root{width:100px}button{width:10px;height:10px;min-width:0;min-height:0;padding:0;border-width:0}button:nth-child(odd){width:20px}button:nth-child(2){width:30px}');
    document.adoptedStyleSheets = [sheet];
    root.innerHTML = ' \n<button></button> \n<button></button> \n<button></button> \n';
    [a,b,c] = Array.from(root.children);
    values = [widths()]; root.appendChild(a); values.push(widths());
    b.remove(); values.push([a.getBoundingClientRect().width,c.getBoundingClientRect().width]);
    root.appendChild(b); values.push(widths());
    output = document.createElement('p'); output.textContent = JSON.stringify(values); root.append(output);
  },
  update() {
    rootNode.replaceChildren(b,c,a,output);
    values.push(widths()); output.textContent = JSON.stringify(values);
  },
});
