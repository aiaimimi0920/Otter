//! Parent-side cross-thread Shader request: the request owns source code, textures share budgeted pixels, and the trusted bundle is separate from Art data.
use super::*;
use crate::{
    pixels::Pixels,
    shader_angle::{Texture, Uniform},
};

pub struct OwnedTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Pixels,
}
pub struct OwnedInvocation {
    pub bundle: Arc<TrustedBundle>,
    pub backend: Backend,
    pub width: u32,
    pub height: u32,
    pub vertex: String,
    pub fragment: String,
    pub textures: Vec<OwnedTexture>,
    pub uniforms: Vec<(String, f32)>,
}
impl OwnedInvocation {
    /// Reuse wire admission before starting the thread; do not extend borrows by copying entire images or serializing large buffers.
    pub fn validate(&self) -> io::Result<()> {
        self.with_invocation(|value| crate::shader_worker_wire::Header::for_invocation(&value))??;
        Ok(())
    }
    pub fn render(&self, stopped: &impl Fn() -> bool) -> Result<Frame, Failure> {
        self.with_invocation(|value| super::render(value, stopped))?
    }
    fn with_invocation<T>(&self, run: impl FnOnce(Invocation<'_>) -> T) -> io::Result<T> {
        if self.textures.len() > 32 || self.uniforms.len() > 256 {
            return Err(crate::shader_worker_wire::invalid());
        }
        let allocation = |_| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                "Shader descriptor allocation failed",
            )
        };
        let mut textures = Vec::new();
        textures
            .try_reserve_exact(self.textures.len())
            .map_err(allocation)?;
        textures.extend(self.textures.iter().map(|item| Texture {
            name: &item.name,
            width: item.width,
            height: item.height,
            rgba: &item.rgba,
        }));
        let mut uniforms = Vec::new();
        uniforms
            .try_reserve_exact(self.uniforms.len())
            .map_err(allocation)?;
        uniforms.extend(self.uniforms.iter().map(|(name, value)| Uniform {
            name,
            value: *value,
        }));
        Ok(run(Invocation {
            bundle: &self.bundle,
            backend: self.backend,
            width: self.width,
            height: self.height,
            request: Request {
                vertex: &self.vertex,
                fragment: &self.fragment,
                textures: &textures,
                uniforms: &uniforms,
            },
        }))
    }
}
