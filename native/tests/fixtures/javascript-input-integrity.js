// Local entries must not read plugin-overridable value/event/collection methods or Promise species.
NeuroSurface.define({ mount({ root }) {
  const status=document.createElement('p'), field=document.createElement('input');
  status.textContent='integrity-ready'; root.append(status,field);
  const read=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').get.bind(field);
  field.addEventListener('input',event=>{if(!event.isTrusted)throw Error('trust');status.textContent=read();});
  const poisoned=()=>{throw Error('public intrinsic invoked');};
  Object.defineProperty(HTMLInputElement.prototype,'value',{get:poisoned,set:poisoned});
  String.prototype.charCodeAt=poisoned;
  WeakMap.prototype.get=WeakMap.prototype.set=poisoned;
  EventTarget.prototype.dispatchEvent=poisoned;
  Promise.prototype.then=poisoned;
  Object.defineProperty(Promise,Symbol.species,{get:poisoned});
  Object.defineProperty(Promise.prototype,'constructor',{get:poisoned});
} });
