// Product-chain fixture: resource reads and business buttons use only the public Art SDK, never test hosts or credentials.
let config, picture, binary;
NeuroSurface.define({mount({root,snapshot}) {
  const resources = snapshot.state.package;
  config = JSON.parse(NeuroSurface.resource(resources.config.resourceId));
  binary = NeuroSurface.resource(resources.binary.resourceId);
  picture = NeuroSurface.image(resources.picture.resourceId);
  if (binary !== 'data:application/octet-stream;base64,AAEC/w==' ||
      picture.width !== 2 || picture.height !== 2) throw Error('package mismatch');
  const button = document.createElement('button');
  button.textContent = 'Run packaged Art';
  button.addEventListener('click', async () => {
    let mode = 'online';
    try {
      const result = await NeuroSurface.service('capability:vendor:service.run', {answer:config.answer}, 15000);
      if (result !== null) throw Error('expected null');
    } catch (error) {
      if (error.message !== 'offline') throw error;
      mode = 'offline';
    }
    const canvas = document.createElement('canvas');
    canvas.id = 'formal'; canvas.width = 2; canvas.height = 2;
    root.append(canvas); canvas.getContext('2d').drawImage(picture, 0, 0);
    NeuroSurface.commitResult({output_image:{kind:'canvas',nodeId:'formal'},
      answer:{kind:'value',value:config.answer},mode:{kind:'value',value:mode}});
  });
  root.replaceChildren(button);
}});
