//! 在既有执行 worker 中从同份准备输入与正式取码结果构造纹理；不创建线程、队列或临时文件。
pub use super::shader_payload::Error;
use super::{loom_input_job::PreparedBatch, shader_payload::Payload, Apartment};
use crate::{
    shader_angle::Backend,
    shader_process::{OwnedInvocation, OwnedTexture, TrustedBundle},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Runtime {
    pub(super) cached: Option<Arc<super::shader_recipe_cache::Recipe>>,
    reference_port: Option<String>,
    pub(super) bundle: Option<Arc<TrustedBundle>>,
    backend: Backend,
    input_port: String,
    uniforms: Vec<(String, f32)>,
    textures: std::collections::BTreeMap<String, String>,
    prepared_textures: Vec<String>,
}
/// 无 bundle 为原生 Auto；显式旧 bundle 仅由可信本地安装 owner 提供，不按运行失败回退。
pub struct Installation {
    pub bundle: Option<Arc<TrustedBundle>>,
    pub backend: Backend,
}
impl Installation {
    pub fn native() -> Self {
        Self {
            bundle: None,
            backend: Backend::D3d11,
        }
    }
    pub(super) fn capture(
        &self,
        capability: &crate::loom_capability::Capability<'_>,
        ports: &[&str],
        parameters: &crate::graph_params::Params<'_>,
        explicit: &[String],
    ) -> Result<Runtime, Error> {
        let input = capability
            .available_shader_input(ports)
            .ok_or(Error::Bindings)?;
        let prepared = super::shader_parameters::prepared_parameters(
            capability, parameters, input, ports, explicit,
        )?;
        let bindings = super::shader_parameters::Bindings::capture(parameters, prepared)?;
        let mut runtime = Runtime::new(
            self.bundle.clone(),
            self.backend,
            input.into(),
            bindings.uniforms,
        )?;
        runtime.textures = bindings.textures;
        // 同名公共图片参数走原图输入准备 owner；连接图优先于参数内联图和响应初值。
        // 未声明名称也必须由应用显式捕获 Unit 来源；reference/主图不提升为辅助纹理。
        runtime.prepared_textures = bindings.prepared;
        runtime.reference_port = capability.shader_requires_reference().then(|| {
            capability
                .shader_reference_port()
                .unwrap_or("reference")
                .into()
        });
        Ok(runtime)
    }
}
impl Runtime {
    /// Some(bundle) 必须由可信安装 owner 提供；None 只使用原生 Auto，不读取 Art 的 DLL 路径。
    pub fn new(
        bundle: Option<Arc<TrustedBundle>>,
        backend: Backend,
        input_port: String,
        uniforms: Vec<(String, f32)>,
    ) -> Result<Self, Error> {
        if input_port.is_empty()
            || input_port.len() > 256
            || input_port.chars().any(char::is_control)
            || uniforms.len() > 256
        {
            return Err(Error::Bindings);
        }
        for (index, (name, value)) in uniforms.iter().enumerate() {
            if crate::shader_angle::name(name).is_err()
                || !value.is_finite()
                || uniforms[..index]
                    .iter()
                    .any(|(previous, _)| previous == name)
            {
                return Err(Error::Bindings);
            }
        }
        Ok(Self {
            cached: None,
            reference_port: None,
            bundle,
            backend,
            input_port,
            uniforms,
            textures: Default::default(),
            prepared_textures: Vec::new(),
        })
    }
    #[cfg(test)]
    pub(super) fn prepare(
        self,
        batch: super::loom_output_batch::Batch,
        inputs: &PreparedBatch,
        cancel: &AtomicBool,
    ) -> Result<OwnedInvocation, Error> {
        self.acquire(batch, inputs, cancel)
            .and_then(|(invocation, _)| match invocation {
                super::shader_invocation::Invocation::Legacy(value) => Ok(value),
                _ => Err(Error::Unsupported),
            })
    }
    pub(super) fn mark_context(&self, sources: &mut [super::loom_input_job::Source]) {
        if let Some(reference) = &self.reference_port {
            for source in sources {
                source.fingerprint_for(&self.input_port);
                source.fingerprint_for(reference);
            }
        }
    }
    pub(super) fn context(
        &self,
        inputs: &PreparedBatch,
    ) -> Option<super::shader_recipe_cache::Context> {
        use super::shader_recipe_cache::Context;
        Some(match &self.reference_port {
            None => Context::Independent,
            Some(reference) => Context::Referenced {
                input: inputs.fingerprint(&self.input_port)?,
                reference: inputs.fingerprint(reference)?,
            },
        })
    }
    pub(super) fn is_cached(&self) -> bool {
        self.cached.is_some()
    }
    pub(super) fn prepare_cached(
        &self,
        inputs: &PreparedBatch,
        cancel: &AtomicBool,
    ) -> Result<super::shader_invocation::Invocation, Error> {
        let recipe = self.cached.as_ref().ok_or(Error::Admission)?;
        self.prepare_payload(&recipe.payload, inputs, cancel)
    }
    pub(super) fn acquire(
        self,
        batch: super::loom_output_batch::Batch,
        inputs: &PreparedBatch,
        cancel: &AtomicBool,
    ) -> Result<
        (
            super::shader_invocation::Invocation,
            Option<super::shader_recipe_cache::Acquired>,
        ),
        Error,
    > {
        check(cancel)?;
        let payload = Arc::new(Payload::from_batch(batch)?);
        let invocation = self.prepare_payload(&payload, inputs, cancel)?;
        let recipe = self
            .context(inputs)
            .map(|context| super::shader_recipe_cache::Acquired { payload, context });
        Ok((invocation, recipe))
    }
    fn prepare_payload(
        &self,
        payload: &Payload,
        inputs: &PreparedBatch,
        cancel: &AtomicBool,
    ) -> Result<super::shader_invocation::Invocation, Error> {
        check(cancel)?;
        if self.bundle.is_none() != payload.is_native() {
            return Err(Error::Unsupported);
        }
        let mut response_textures: std::collections::BTreeMap<_, _> = payload
            .textures
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        // 单次快照按旧参数覆盖优先级安装，避免两个异步 texture load 的完成顺序竞争。
        for (name, uri) in &self.textures {
            response_textures.insert(name, uri);
        }
        for name in &self.prepared_textures {
            response_textures.remove(name.as_str());
        }
        if response_textures.len() + self.prepared_textures.len() > 31 {
            return Err(Error::Bindings);
        }
        let (_, descriptor) = inputs
            .descriptors()
            .find(|(port, _)| *port == self.input_port)
            .ok_or(Error::Bindings)?;
        let _apartment = Apartment::new().map_err(|_| Error::Decode)?;
        let input = decode_input(
            descriptor,
            crate::shader_worker_wire::MAX_TEXTURE_BYTES,
            cancel,
        )?;
        let (width, height) = (input.width, input.height);
        let mut total = input.rgba.len();
        let mut textures = Vec::new();
        textures
            .try_reserve_exact(response_textures.len() + self.prepared_textures.len() + 1)
            .map_err(|_| Error::Budget)?;
        textures.push(texture("input".into(), input));
        for name in &self.prepared_textures {
            let (_, descriptor) = inputs
                .descriptors()
                .find(|(port, _)| *port == name)
                .ok_or(Error::Bindings)?;
            let decoded = decode_input(
                descriptor,
                crate::shader_worker_wire::MAX_TEXTURE_BYTES - total,
                cancel,
            )?;
            total = total
                .checked_add(decoded.rgba.len())
                .filter(|total| *total <= crate::shader_worker_wire::MAX_TEXTURE_BYTES)
                .ok_or(Error::Budget)?;
            textures.push(texture(name.clone(), decoded));
        }
        for (name, uri) in response_textures {
            check(cancel)?;
            if uri.is_empty() {
                continue;
            }
            let encoded = uri
                .strip_prefix("data:image/png;base64,")
                .ok_or(Error::Unsupported)?;
            let decoded = decode(
                encoded,
                crate::shader_worker_wire::MAX_TEXTURE_BYTES - total,
                cancel,
            )?;
            total = total
                .checked_add(decoded.rgba.len())
                .filter(|total| *total <= crate::shader_worker_wire::MAX_TEXTURE_BYTES)
                .ok_or(Error::Budget)?;
            textures.push(texture(name.into(), decoded));
        }
        let mut uniforms = payload.uniforms.clone();
        for (name, value) in &self.uniforms {
            uniforms.insert(name.clone(), *value);
        }
        let invocation = if let Some(bundle) = &self.bundle {
            super::shader_invocation::Invocation::Legacy(OwnedInvocation {
                bundle: bundle.clone(),
                backend: self.backend,
                width,
                height,
                vertex: payload.vertex_shader.clone(),
                fragment: payload.fragment_shader.clone(),
                textures,
                uniforms: uniforms.into_iter().collect(),
            })
        } else {
            super::shader_invocation::Invocation::Native(
                super::shader_native_invocation::Invocation {
                    source: payload.source.clone(),
                    width,
                    height,
                    textures: textures
                        .into_iter()
                        .map(|texture| super::shader_native_invocation::Texture {
                            name: texture.name,
                            width: texture.width,
                            height: texture.height,
                            rgba: texture.rgba,
                        })
                        .collect(),
                    uniforms: uniforms.into_iter().collect(),
                },
            )
        };
        invocation.validate().map_err(|_| Error::Admission)?;
        check(cancel)?;
        Ok(invocation)
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn decode_input(
    descriptor: crate::loom_art_wire::Input<'_>,
    remaining: usize,
    cancel: &AtomicBool,
) -> Result<crate::image_decode::shader::Texture, Error> {
    let crate::loom_art_wire::Input::InlinePng {
        base64,
        width,
        height,
    } = descriptor
    else {
        return Err(Error::Unsupported);
    };
    let texture = decode(base64, remaining, cancel)?;
    if (width, height) != (texture.width, texture.height) {
        return Err(Error::Dimensions);
    }
    Ok(texture)
}
fn decode(
    encoded: &str,
    remaining: usize,
    cancel: &AtomicBool,
) -> Result<crate::image_decode::shader::Texture, Error> {
    check(cancel)?;
    let bytes = super::loom_output_inline::decode_bytes("image/png", encoded, cancel)
        .map_err(|_| Error::Decode)?;
    let value =
        crate::image_decode::shader::png(&bytes, remaining, &|| cancel.load(Ordering::Acquire))
            .map_err(|_| Error::Decode)?;
    check(cancel)?;
    Ok(value)
}
#[cfg(test)]
#[path = "shader_prepare_tests.rs"]
pub(crate) mod tests;
fn texture(name: String, value: crate::image_decode::shader::Texture) -> OwnedTexture {
    OwnedTexture {
        name,
        width: value.width,
        height: value.height,
        rgba: value.rgba,
    }
}
