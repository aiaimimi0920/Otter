// 通用图形夹具：真实离屏绘制、原生内容盒和 observer→rAF，不包含股票业务或网络。
let canvas, context, label, observer, count = 0, frames = 0, bounds = 0;
const report = () => { label.textContent = JSON.stringify({ count, frames, bounds, same: context === canvas.getContext("2d") }); };
const draw = color => {
  context.setTransform(1,0,0,1,0,0);
  context.clearRect(0,0,80,40);
  context.fillStyle = color; context.fillRect(0,0,80,40);
  context.save(); context.globalAlpha = 0.5; context.fillStyle = "red"; context.fillRect(8,0,8,8); context.restore();
  context.clearRect(72,0,8,8);
  context.fillStyle = "yellow"; context.fillRect(24,8,-8,-8);
  context.beginPath(); context.moveTo(28,4); context.lineTo(44,4);
  context.strokeStyle = "white"; context.lineWidth = 2; context.lineCap = "round"; context.setLineDash([3,3]); context.stroke();
  context.setLineDash([]); context.beginPath(); context.arc(54,16,4,0,Math.PI*2);
  context.fillStyle = "yellow"; context.fill();
  context.font = "12px Consolas, monospace"; context.textAlign = "left"; context.fillStyle = "white";
  context.fillText("Canvas 原生",2,36);
};
NeuroSurface.define({
  mount({ root }) {
    root.style.cssText = "box-sizing:border-box;display:flex;flex-direction:column;padding:0;gap:0";
    canvas = document.createElement("canvas"); canvas.width = 80; canvas.height = 40;
    canvas.style.cssText = "box-sizing:border-box;width:50%;height:40px";
    context = canvas.getContext("2d"); label = document.createElement("p"); root.append(canvas,label);
    draw("#00ff00");
    observer = new ResizeObserver(entries => {
      bounds = entries[0].contentRect.width; count++;
      requestAnimationFrame(() => { frames++; draw(frames % 2 ? "blue" : "#00ff00"); report(); }); report();
    });
    observer.observe(canvas); report();
  },
  update() { draw("red"); report(); },
  dispose() { observer.disconnect(); },
});
