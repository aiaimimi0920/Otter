// 单 operation 有界服务 Promise；宿主桥/resolve 不暴露给脚本，身份仍归父进程。
((json, fail, images, canvas) => {
  "use strict";
  const PromiseType = Promise, ErrorType = Error, integer = Number.isInteger, freeze = Object.freeze;
  const test = Function.prototype.call.bind(RegExp.prototype.test), array = Array.isArray;
  const descriptor = Object.getOwnPropertyDescriptor, own = Object.hasOwn, plain = Object.setPrototypeOf, keys = Object.keys;
  const validName = /^[A-Za-z0-9._:-]{1,256}$/;
  let active = false, preparing = false, sequence = 0, pending = null;
  const selectImages = handles => {
    if (!array(handles) || handles.length < 1 || handles.length > 4) fail();
    const ids = plain([], null);
    for (let i = 0; i < handles.length; i++) {
      const item = descriptor(handles, i);
      if (!item || !own(item, "value")) fail();
      const id = images.resolve(item.value).id;
      for (let j = 0; j < ids.length; j++) if (ids[j] === id) fail();
      ids[ids.length] = id;
    }
    return ids;
  };
  const invoke = (service, input, timeoutMs, imageIds, canvases, configure) => {
    if (!active || preparing || pending || sequence >= 32 || typeof service !== "string" ||
        !test(validName, service) || !integer(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) fail();
    // 私有编码拒绝 getter/toJSON，父侧再次按真实 UTF-8 字节数检查。
    let request;
    preparing = true;
    try {
      const encoded = json.encode(input);
      if (encoded.length > 65536) fail();
      request = { id: sequence + 1, service, input: json.decode(encoded), timeoutMs };
      if (imageIds) request.images = imageIds;
      if (canvases) request.canvases = canvas.serviceFrames(canvases);
      if (configure) configure(request);
      if (!integer(request.timeoutMs) || request.timeoutMs < 1 || request.timeoutMs > 60000) fail();
    } finally { preparing = false; }
    // 参数准备可能触发Proxy trap；禁止重入抢占pending，失败也不制造私有序号缺口。
    sequence++;
    return new PromiseType((resolve, reject) => { pending = { request, resolve, reject }; });
  };
  return freeze({
    call: (service, input, timeoutMs = 60000) => invoke(service, input, timeoutMs),
    callImages(service, input, handles, timeoutMs = 60000) {
      return invoke(service, input, timeoutMs, undefined, undefined, request => { request.images = selectImages(handles); });
    },
    callCanvases(service, input, handles, timeoutMs = 60000) {
      if (!array(handles) || handles.length < 1 || handles.length > 4) fail();
      return invoke(service, input, timeoutMs, undefined, handles);
    },
    callImage(service, input, options = undefined) {
      return invoke(service, input, 60000, undefined, undefined, request => {
        request.imageOutput = true;
        if (options === undefined) return;
        if (!options || typeof options !== "object") fail();
        const names = keys(options);
        for (let i = 0; i < names.length; i++) {
          const name = names[i], item = descriptor(options,name);
          if (!item || !own(item,"value")) fail();
          if (name === "timeoutMs") request.timeoutMs = item.value;
          else if (name === "images") request.images = selectImages(item.value);
          else if (name === "canvases") request.canvases = canvas.serviceFrames(item.value);
          else fail();
        }
        if (request.images && request.canvases) fail();
      });
    },
    callWithConfirmation(service, input, options = undefined) {
      return invoke(service, input, 60000, undefined, undefined, request => {
        request.confirmation = true;
        if (options === undefined) return;
        if (!options || typeof options !== "object") fail();
        const names = keys(options);
        for (let i = 0; i < names.length; i++) {
          const name = names[i], item = descriptor(options,name);
          if (!item || !own(item,"value")) fail();
          if (name === "timeoutMs") request.timeoutMs = item.value;
          else if (name === "images") request.images = selectImages(item.value);
          else if (name === "canvases") request.canvases = canvas.serviceFrames(item.value);
          else if (name === "imageOutput" && typeof item.value === "boolean") request.imageOutput = item.value;
          else fail();
        }
        if (request.images && request.canvases) fail();
      });
    },
    begin(allowed) { if (pending) fail(); active = allowed; sequence = 0; },
    end() { if (pending) fail(); active = false; },
    next() { return json.encode(pending ? pending.request : null); },
    settle(encoded) {
      const reply = json.decode(encoded);
      if (!pending || reply.id !== pending.request.id) fail();
      const owner = pending; pending = null;
      if (reply.result.status === "image" && owner.request.imageOutput)
        owner.resolve(freeze({ output: reply.result.output, image: images.image(reply.result.image.id) }));
      else if (reply.result.status === "succeeded" && !owner.request.imageOutput) owner.resolve(reply.result.output);
      else if (reply.result.status === "failed") owner.reject(new ErrorType(reply.result.error));
      else fail();
    },
  });
})
