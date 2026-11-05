// 私有请求不读取公开反射/方法，也不通过被污染的 collection intrinsic 或 thenable。
NeuroSurface.define({mount({root}){
  root.innerHTML='<p>integrity</p><input><button>request</button>';
  const field=root.children[1],focus=field.focus,blur=field.blur;
  root.children[2].addEventListener('click',()=>{focus.call(field);blur.call(field);});
  const fail=()=>{throw Error('public reflection used');};
  Object.defineProperty(HTMLInputElement.prototype,'disabled',{get:fail});
  Object.defineProperty(HTMLInputElement.prototype,'focus',{value:fail});
  Object.defineProperty(HTMLInputElement.prototype,'blur',{value:fail});
  WeakMap.prototype.get=fail; Map.prototype.get=fail; Array.prototype.push=fail;
}});
