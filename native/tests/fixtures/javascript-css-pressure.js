// 与原256节点/16规则Engine压力同规模；沿持久child观察重复投影，不含业务或外部请求。
NeuroSurface.define({ mount({root}) {
  for(let i=0;i<256;i++){const node=document.createElement('button');node.className='row';root.appendChild(node)}
  const rules=Array.from({length:16},(_,i)=>'.row:nth-child('+(i+1)+'n+1){opacity:.5}').join('');
  const sheet=new CSSStyleSheet();sheet.replaceSync(rules);document.adoptedStyleSheets=[sheet];
} });
