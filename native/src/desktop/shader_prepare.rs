//! Build textures from the same prepared inputs and official code-fetch results in the existing execution worker; no new threads, queues, or temporary files.
pub use super::shader_payload::Error;
use super::{
    loom_input_job::PreparedBatch, shader_native_invocation, shader_payload::Payload, Apartment,
};
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
    scope_id: Option<String>,
    reference_port: Option<String>,
    pub(super) bundle: Option<Arc<TrustedBundle>>,
    backend: Backend,
    input_port: String,
    uniforms: Vec<(String, f32)>,
    textures: std::collections::BTreeMap<String, String>,
    prepared_textures: Vec<String>,
}
/// No bundle means native Auto; only a trusted local-installation owner supplies explicit legacy bundles, never as runtime-failure fallback.
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
        // Same-named public image parameters use the original-image input owner; connected images outrank parameter inline images and response defaults.
        // Even undeclared names require application-captured Unit sources; reference/main images are not promoted to auxiliary textures.
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
    pub(super) fn bind_scope(
        &mut self,
        target: &crate::workspace_id::UnitId,
        selection: &crate::workspace_id::UnitId,
    ) {
        // Update Selection.identity on creation/restore/rebinding; prefix lengths eliminate concatenation ambiguity for arbitrary legacy UnitIds.
        // Do not use Art IDs as node scope or impose session-key length limits on valid long Art IDs.
        self.scope_id = Some(format!(
            "{}:{}{}",
            target.as_str().len(),
            target.as_str(),
            selection.as_str()
        ));
    }
    /// Some(bundle) must come from a trusted installation owner; None uses only native Auto and never reads Art DLL paths.
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
            scope_id: None,
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
        // Install one snapshot using legacy parameter-override precedence, avoiding completion-order races between two asynchronous texture loads.
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
                textures: textures
                    .into_iter()
                    .map(|texture| OwnedTexture {
                        name: texture.name,
                        width: texture.width,
                        height: texture.height,
                        rgba: texture.rgba,
                    })
                    .collect(),
                uniforms: uniforms.into_iter().collect(),
            })
        } else {
            super::shader_invocation::Invocation::Native(shader_native_invocation::Invocation {
                scope: self.scope_id.clone(),
                source: payload.source.clone(),
                width,
                height,
                textures,
                uniforms: uniforms.into_iter().collect(),
            })
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
    let bytes = decode_result(
        super::loom_output_inline::decode_bytes("image/png", encoded, cancel),
        cancel,
    )?;
    decode_result(
        crate::image_decode::shader::png(&bytes, remaining, &|| cancel.load(Ordering::Acquire)),
        cancel,
    )
}

fn decode_result<T>(result: windows::core::Result<T>, cancel: &AtomicBool) -> Result<T, Error> {
    // Both decoders can return an ordinary error after observing cancellation.
    // Check before classifying the result or passing a texture to the next stage.
    check(cancel)?;
    result.map_err(|_| Error::Decode)
}
#[cfg(test)]
#[path = "shader_prepare_tests.rs"]
pub(crate) mod tests;
fn texture(
    name: String,
    value: crate::image_decode::shader::Texture,
) -> shader_native_invocation::Texture {
    shader_native_invocation::Texture {
        name,
        width: value.width,
        height: value.height,
        rgba: value.rgba,
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    use crate::{graph_params::Params, loom_capability::Capability, workspace_id::UnitId};

    #[test]
    fn native_scope_uses_node_and_selection_generation_not_shared_or_long_art_id() {
        let raw = serde_json::json!({
            "id": "art".repeat(200),
            "inputs": [{"name": "input_path", "type": "image"}]
        });
        let capability = Capability::new(&raw).unwrap();
        let parameters = Params::new(&[], &[]).unwrap();
        let capture = || {
            Installation::native()
                .capture(&capability, &["input_path"], &parameters, &[])
                .unwrap()
        };
        let mut first = capture();
        assert!(
            first.scope_id.is_none(),
            "no binding must not fabricate an Art scope"
        );
        let node_a = UnitId::parse("node-a").unwrap();
        let node_b = UnitId::parse("node-b").unwrap();
        let binding_a = UnitId::parse("binding-a").unwrap();
        let binding_b = UnitId::parse("binding-b").unwrap();
        first.bind_scope(&node_a, &binding_a);
        let mut second = capture();
        second.bind_scope(&node_b, &binding_a);
        assert_ne!(
            first.scope_id, second.scope_id,
            "same Art on different nodes is isolated"
        );
        second.bind_scope(&node_a, &binding_b);
        assert_ne!(
            first.scope_id, second.scope_id,
            "new owner generation is isolated"
        );
        second.bind_scope(&node_a, &binding_a);
        assert_eq!(first.scope_id, second.scope_id);
        let longest = UnitId::parse(&"n".repeat(256)).unwrap();
        second.bind_scope(&longest, &longest);
        assert!(second.scope_id.as_ref().unwrap().len() <= 1024);
    }
}
