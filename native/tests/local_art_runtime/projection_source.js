// 公开SDK的预览与正式结果分界；不读取测试参数、宿主内部、文件或凭据。
NeuroSurface.define({mount({root}) {
  const canvas = document.createElement('canvas');
  canvas.id = 'projection-formal';
  canvas.width = 2;
  canvas.height = 2;
  const draw = (color) => {
    const context = canvas.getContext('2d');
    context.fillStyle = color;
    context.fillRect(0, 0, 2, 2);
  };
  const button = (name, color, mode) => {
    const node = document.createElement('button');
    node.textContent = name;
    node.addEventListener('click', () => {
      draw(color);
      node.textContent = name + ' submitted';
      if (mode) NeuroSurface.commitResult({
        output_image: {kind:'canvas', nodeId:'projection-formal'},
        mode: {kind:'value', value:mode}
      });
    });
    return node;
  };
  root.replaceChildren(
    button('Commit Red', '#ff0000', 'red'),
    button('Preview Blue', '#0000ff', null),
    button('Commit Green', '#00ff00', 'green'),
    button('Commit Blue', '#0000ff', 'blue'),
    canvas
  );
}});
