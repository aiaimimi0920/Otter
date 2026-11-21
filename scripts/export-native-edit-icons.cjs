// Export the existing, user-tuned toolbar SVGs verbatim; this is not an icon redesign.
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const components = path.join(root, 'src/features/stickers/components');
const destination = path.join(root, 'native/assets/edit-strip');
function component(file, name, base = components) {
  const source = fs.readFileSync(path.join(base, file), 'utf8');
  const declaration = source.indexOf(`const ${name}`);
  if (declaration < 0) throw new Error(`Missing baseline icon ${name}`);
  const start = source.indexOf('<svg', declaration);
  const end = source.indexOf('</svg>', start) + 6;
  if (start < 0 || end < 6) throw new Error(`Missing SVG ${name}`);
  return source.slice(start, end).replace(/\sclass=\{props\.class\}/g, '')
    .replace(/\sclass="[^"]*"/g, '').replace(/\saria-hidden="true"/g, '');
}
function exportsFromBaseline() {
  const names = ['SelectMode', 'MoveMode', 'RotateMode', 'ScaleMode', 'RectTool',
    'EllipseTool', 'TriangleTool', 'PolygonTool', 'LineTool', 'BrushTool', 'TextTool',
    'SerialTool', 'MosaicTool', 'BlurTool', 'EraserTool', 'CropTool', 'UndoTool',
    'RedoTool', 'RasterizeSelectedTool', 'RasterizeAllTool'];
  const icons = Object.fromEntries(names.map(name => [name, component('stickerTopStripIcons.tsx', `${name}Icon`)]));
  icons.BezierTool = component('stickerTopStripCatalog.tsx', 'BezierToolIcon');
  icons.Paintbrush = component('StickerTopStripCreateTools.tsx', 'PaintbrushIcon');
  icons.Settings = component('StickerSettingsFields.tsx', 'StickerSettingsIcon');
  const loom = fs.readFileSync(path.join(root, 'src/integrations/loom/LoomConnectionSettings.tsx'), 'utf8');
  const loomSvg = loom.match(/<svg viewBox="0 0 1024 1024"[\s\S]*?<\/svg>/);
  if (!loomSvg) throw new Error('Loom settings icon baseline changed');
  icons.SettingsLoom = loomSvg[0].replace('var(--theme-signal, #d9ff38)', '#d9ff38');
  const projection = fs.readFileSync(path.join(root, 'src/features/projection/components/ProjectionSettingsIcons.tsx'), 'utf8');
  const shell = projection.match(/<svg\b[\s\S]*?aria-hidden="true">/);
  if (!shell) throw new Error('Projection settings SVG baseline changed');
  for (const [name, kind] of [['Send', 'send'], ['Receive', 'receive'], ['Flow', 'flow']]) {
    const pattern = kind === 'flow' ? /<Show when=\{props.kind === "flow"\}>([\s\S]*?)<\/Show>/
      : /<Show when=\{props.kind === "send" \|\| props.kind === "receive"\}>([\s\S]*?)<\/Show>/;
    const body = projection.match(pattern);
    if (!body) throw new Error(`Projection settings ${name} baseline changed`);
    icons[`Settings${name}`] = shell[0] + body[1].replace(/d=\{props.kind === "send" \? "([^"]+)" : "([^"]+)"\}/,
      (_, send, receive) => `d="${kind === 'send' ? send : receive}"`) + '</svg>';
  }
  for (const [name, kind] of [['Import', 'import'], ['Export', 'export'], ['Source', 'source'], ['Copy', 'copy'], ['Unlink', 'unlink']]) {
    const pattern = ['import', 'export'].includes(kind)
      ? /<Show when=\{props.kind === "import" \|\| props.kind === "export"\}>([\s\S]*?)<\/Show>/
      : new RegExp(`<Show when=\\{props.kind === "${kind}"\\}>([\\s\\S]*?)<\\/Show>`);
    const body = projection.match(pattern);
    if (!body) throw new Error(`Projection settings ${name} baseline changed`);
    icons[`Settings${name}`] = shell[0] + body[1].replace(/d=\{props.kind === "import" \? "([^"]+)" : "([^"]+)"\}/,
      (_, incoming, outgoing) => `d="${kind === 'import' ? incoming : outgoing}"`) + '</svg>';
  }
  const toolbar = fs.readFileSync(path.join(root, 'src/features/projection/components/ProjectionToolbar.tsx'), 'utf8');
  const toolbarShell = toolbar.match(/<svg\b[\s\S]*?aria-hidden="true">/)[0].replace(/\sclass=\{[^}]+\}/, '');
  for (const [name, kind] of [['OneWay', 'flow'], ['TwoWay', 'flow-two'], ['Qr', 'qr'], ['Link', 'link'], ['Devices', 'devices'], ['Groups', 'groups'], ['Friends', 'friends']]) {
    const pattern = kind === 'devices' ? /<Show when=\{props.kind === "devices" \|\| props.kind === "source"\}>([\s\S]*?)<\/Show>/
      : new RegExp(`<Show when=\\{props.kind === "${kind}"\\}>([\\s\\S]*?)<\\/Show>`);
    const body = toolbar.match(pattern);
    if (!body) throw new Error(`Projection toolbar ${name} baseline changed`);
    icons[`Settings${name}`] = toolbarShell + body[1] + '</svg>';
    if (kind === 'flow') icons.SettingsOneWayReceive = toolbarShell + body[1].replace('<path', '<path transform="rotate(180 12 12)"') + '</svg>';
  }
  icons.SettingsBorder = component('stickerTopStripPropertyBarIcons.tsx', 'StrokeColorIcon');
  icons.SettingsRadius = component('stickerTopStripPropertyBarIcons.tsx', 'RadiusIcon');
  for (const name of ['LineWidth', 'Radius', 'Step', 'Opacity', 'ArrowHead', 'AngleSnap', 'HighlighterGlow', 'AnnotationsOnlyFocused']) {
    icons[`Property${name}`] = component('stickerTopStripPropertyBarIcons.tsx', `${name}Icon`);
  }
  const inline = (file, base = components) => [...fs.readFileSync(path.join(base, file), 'utf8')
    .matchAll(/<svg\b[^>]*>[\s\S]*?<\/svg>/g)].map(m => m[0]
      .replace(/\sclass="unit-trigger-glyph"/g, ' fill="currentColor" stroke-width="1"')
      .replace(/\sclass="[^"]*"/g, '').replace(/\saria-hidden="true"/g, ''));
  const shape = inline('RectanglePropertyFields.tsx');
  if (shape.length !== 8) throw new Error('Shape property baseline changed');
  for (const [i, name] of ['LineWidth', 'Radius', 'Step', 'PolygonSides'].entries()) icons[`Property${name}`] = shape[i];
  icons.PropertyConstrain = shape[7];
  icons.PropertyAspectRatio = inline('RectangleAspectRatioField.tsx')[0];
  const brush = inline('BrushPropertyFields.tsx');
  if (brush.length !== 3) throw new Error('Brush property baseline changed');
  for (const [i, name] of ['Opacity', 'Smoothing', 'HighlighterGlow'].entries()) icons[`Property${name}`] = brush[i];
  const text = inline('TextSerialPropertyFields.tsx');
  if (text.length !== 1) throw new Error('Text property baseline changed');
  const variant = /<Show when=\{props\.serial\} fallback=\{(<path[^>]+\/>)}>([\s\S]*?)<\/Show>/;
  if (!variant.test(text[0])) throw new Error('Text property variants changed');
  icons.PropertyTextSize = text[0].replace(variant, '$1');
  icons.PropertySerialRadius = text[0].replace(variant, '$2');
  const actions = fs.readFileSync(path.join(components, 'StickerTopStripTextActions.tsx'), 'utf8');
  const svgs = [...actions.matchAll(/<svg\b[^>]*>[\s\S]*?<\/svg>/g)];
  if (svgs.length !== 2) throw new Error('Text overlay baseline changed');
  for (const [i, name] of ['Ocr', 'Translate'].entries()) {
    icons[name] = svgs[i][0].replace(/\sclass="[^"]*"/g, '').replace(/\saria-hidden="true"/g, '');
  }
  const picker = inline('ColorPicker.tsx', path.join(root, 'src/features/workspace/components'));
  if (picker.length !== 3) throw new Error('Color picker baseline changed');
  for (const [i, name] of ['Eyedropper', 'Remove', 'Apply'].entries()) icons[`Color${name}`] = picker[i];
  for (const name of ['FlipX', 'FlipY', 'Opacity']) icons[`Settings${name}`] = component('stickerTopStripPropertyBarIcons.tsx', `${name}Icon`);
  const settings = inline('StickerSettingsFields.tsx');
  if (settings.length !== 5) throw new Error('Sticker settings baseline changed');
  icons.SettingsWidth = settings[3];
  icons.SettingsHeight = settings[4];
  const fit = inline('FitImageFrameButton.tsx');
  if (fit.length !== 1) throw new Error('Fit frame baseline changed');
  icons.SettingsFit = fit[0];
  const extras = inline('StickerSettingsExtras.tsx');
  const extraPaths = /d=\{action\.id === "ocr" \? "([^"]+)"\s*: "([^"]+)"\}/;
  if (extras.length !== 1 || !extraPaths.test(extras[0])) throw new Error('Settings text-copy baseline changed');
  for (const [i, name] of ['Ocr', 'Translation'].entries()) {
    icons[`SettingsCopy${name}`] = extras[0].replace(extraPaths, (_, ocr, translation) => `d="${i === 0 ? ocr : translation}"`);
  }
  const portSource = fs.readFileSync(path.join(root, 'src/features/workspace/components/UnitPropertyTypeIcon.tsx'), 'utf8');
  const portTypes = [...portSource.matchAll(/^\s+(\w+): \["(#[a-fA-F0-9]{6})", "([^"]+)"\]/gm)];
  if (portTypes.length !== 7) throw new Error('Property type baseline changed');
  for (const [, name, color, shape] of portTypes) {
    icons[`Port${name[0].toUpperCase()}${name.slice(1)}`] = `<svg viewBox="0 0 24 24" fill="${color}"><path d="${shape}" /></svg>`;
  }
  for (const name of ['Visible', 'Hidden']) {
    icons[`Port${name}`] = component('UnitParamsPortRows.tsx', `${name}PortIcon`, path.join(root, 'src/features/art/components'));
  }
  const drawer = inline('UnitNodeSettingsStrip.tsx', path.join(root, 'src/features/art/components'));
  if (drawer.length !== 7) throw new Error('Node drawer baseline changed');
  const slash = /<Show when=\{props\.unit\.data\.hideCollapsedEndpoints\}>([\s\S]*?)<\/Show>/;
  if (!slash.test(drawer[0])) throw new Error('Node endpoints baseline changed');
  icons.NodeEndpoints = drawer[0].replace(slash, '');
  icons.NodeEndpointsHidden = drawer[0].replace(slash, '$1');
  for (const [i, name] of ['Upstream', 'Parameter', 'Notify', 'Relay', 'Run'].entries()) {
    icons[`Node${name}`] = drawer[i + 1];
  }
  for (const name of Object.keys(icons).filter(name => name.startsWith('Node'))) {
    icons[name] = icons[name].replace(/<svg\b/, '<svg fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"');
  }
  return Object.fromEntries(Object.entries(icons).map(([name, svg]) => {
    if (/[{}]/.test(svg)) throw new Error(`Dynamic baseline SVG ${name}`);
    // Accessible names belong to the native BUTTON; web ARIA metadata is not SVG geometry.
    svg = svg.replace(/\s(?:aria-label|aria-hidden|role)="[^"]*"/g, '');
    return [`${name}.svg`, `${svg.replace(/[ \t]+$/gm, '')}\n`];
  }));
}
function run(check) {
  const icons = exportsFromBaseline();
  if (!check) fs.mkdirSync(destination, { recursive: true });
  for (const [name, svg] of Object.entries(icons)) {
    const file = path.join(destination, name);
    if (check) {
      // Checkout EOL conversion must not look like geometry drift.
      if (fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n') !== svg) throw new Error(`Baseline SVG drift: ${name}`);
    } else fs.writeFileSync(file, svg, 'utf8');
  }
  return Object.keys(icons).length;
}
if (require.main === module) console.log(`Baseline toolbar SVGs: ${run(process.argv.includes('--check'))}`);
module.exports = { exportsFromBaseline, run };
