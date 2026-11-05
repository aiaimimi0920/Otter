// 通用输入转换中同步读写几何；不依赖模型计数、私有collect或宿主业务。
NeuroSurface.define({ mount({ root }) {
  root.style.cssText = "box-sizing:border-box;width:40px";
  if (root.clientWidth !== 40) throw Error("initial geometry");
  let conversions = 0;
  root.style.cssText = { toString() {
    conversions++;
    if (root.clientWidth !== 40) throw Error("preconversion geometry");
    root.style.width = "50px";
    if (root.clientWidth !== 50) throw Error("reentrant mutation geometry");
    return "box-sizing:border-box;width:60px;color:green";
  } };
  if (conversions !== 1 || root.clientWidth !== 60) throw Error("committed geometry");
  root.textContent = "ready";
} });
