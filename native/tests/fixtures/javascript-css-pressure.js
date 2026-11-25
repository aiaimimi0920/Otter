// Match the original 256-node/16-rule Engine pressure test; observe repeated projection through persistent children, without business logic or external requests.
NeuroSurface.define({ mount({root}) {
  for(let i=0;i<256;i++){const node=document.createElement('button');node.className='row';root.appendChild(node)}
  const rules=Array.from({length:16},(_,i)=>'.row:nth-child('+(i+1)+'n+1){opacity:.5}').join('');
  const sheet=new CSSStyleSheet();sheet.replaceSync(rules);document.adoptedStyleSheets=[sheet];
} });
