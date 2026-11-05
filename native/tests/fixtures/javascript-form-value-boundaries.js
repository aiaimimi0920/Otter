// 同源 child/实际产品共用；每次点击只验证一个边界，仍受原operation总期限约束。
let index = 0;
const cases = [
  () => ['', ''],
  () => ['ascii\t\u007f', 'ascii\t\u007f'],
  () => ['a\r\nb\nc\rd', 'abcd'],
  () => ['x'.repeat(48 * 1024), 'x'.repeat(48 * 1024)],
  () => ['x'.repeat(48 * 1024 + 1), null],
  () => ['中'.repeat(16 * 1024), '中'.repeat(16 * 1024)],
  () => ['中'.repeat(16 * 1024 + 1), null],
  () => ['🙂'.repeat(12 * 1024), '🙂'.repeat(12 * 1024)],
  () => ['🙂'.repeat(12 * 1024 + 1), null],
  () => ['\u07ff'.repeat(24 * 1024), '\u07ff'.repeat(24 * 1024)],
  () => ['\u07ff'.repeat(24 * 1024 + 1), null],
  () => ['a\n'.repeat(32768), 'a'.repeat(32768)],
  () => ['\r\n'.repeat(32768), ''],
  () => ['\n'.repeat(65537), null],
  () => ['a\0b', null],
  () => ['\uD800', null],
  () => ['\uDC00', null],
  () => ['\uD800\n\uDC00', null],
  () => ['\uD800\uD800\uDC00', null],
  () => ['\uDC00\uD800', null],
  () => ['\u007f\u0080\u07ff\u0800\uD7FF\uE000\uFFFF🙂', '\u007f\u0080\u07ff\u0800\uD7FF\uE000\uFFFF🙂'],
  () => ['\r🙂\n中\r\n', '🙂中'],
  () => ['a\r'.repeat(32768), 'a'.repeat(32768)],
  () => ['a\r\n'.repeat(21845) + 'a', 'a'.repeat(21846)],
  () => ['x'.repeat(48 * 1024) + '\n'.repeat(16 * 1024), 'x'.repeat(48 * 1024)],
  () => ['x'.repeat(48 * 1024 + 1) + '\r', null],
  () => ['\n' + '中'.repeat(16 * 1024) + '\r', '中'.repeat(16 * 1024)],
  () => ['\r\uD800\n\uDC00\n', null],
];
NeuroSurface.define({
  mount({ root }) {
    root.innerHTML = '<p>ready</p><input value="seed"><button>next</button>';
    const status = root.children[0], field = root.children[1];
    root.children[2].addEventListener('click', () => {
      const [raw, expected] = cases[index]();
      const numeric = Object.getOwnPropertyDescriptor(Array.prototype, '0');
      const hooks = [[String.prototype, Symbol.split], [RegExp.prototype, Symbol.split], [Object.prototype, Symbol.split],
        [Object.prototype, Symbol.toPrimitive], [Object.prototype, 'toString'], [Object.prototype, 'valueOf'],
        [Array.prototype, '32768'], [Array.prototype, 'constructor'], [Array, Symbol.species]];
      const saved = hooks.map(([prototype, key]) => Object.getOwnPropertyDescriptor(prototype, key));
      for (const [prototype, key] of hooks) Object.defineProperty(prototype, key, { get() { throw Error('symbol hook'); }, configurable: true });
      Object.defineProperty(Array.prototype, '0', { get() { throw Error('array getter'); }, set() { throw Error('array setter'); }, configurable: true });
      try {
        field.value = 'preserved';
        let rejected = false;
        try { field.value = raw; } catch { rejected = true; }
        if (rejected !== (expected === null) || field.value !== (expected ?? 'preserved')) throw Error('value boundary');
        if (field.defaultValue !== 'seed') throw Error('dirty default changed');
        field.value = 'seed';
      } finally {
        // QuickJS自身JSON.stringify会写原型数值槽；只在值校验期间注入，不改产品引擎。
        if (numeric) Object.defineProperty(Array.prototype, '0', numeric); else delete Array.prototype[0];
        for (let i = 0; i < hooks.length; i++) {
          const [prototype, key] = hooks[i];
          if (saved[i]) Object.defineProperty(prototype, key, saved[i]); else delete prototype[key];
        }
      }
      status.textContent = `case:${++index}`;
    });
    // 新快路径和保留段复制不能转调公开方法；数值槽污染仅覆盖被测值校验调用。
    String.prototype.charCodeAt = String.prototype.slice = String.prototype.split = String.prototype.replace =
      String.prototype.replaceAll = Array.prototype.join = RegExp.prototype.exec = () => { throw Error('public intrinsic'); };
  },
});
