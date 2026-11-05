//! child 私有离屏 Canvas owner；软件 D2D/DIB 不跨线程，背板/提交/PNG 均有界且无文件 I/O。
use crate::surface_javascript_wire::{canvas, invalid};
use std::{collections::BTreeMap, io, sync::Arc, time::Instant};
use windows::{
    core::Result,
    Win32::{Foundation::RPC_E_CHANGED_MODE, Graphics::Direct2D::*, System::Com::*},
};
#[path = "surface_javascript_canvas_backing.rs"]
mod backing;
#[path = "surface_javascript_canvas_commands.rs"]
mod commands;
#[path = "surface_javascript_canvas_draw.rs"]
mod draw;
#[path = "surface_javascript_canvas_images.rs"]
mod images;
#[path = "surface_javascript_canvas_path.rs"]
mod path;
use backing::Backing;
macro_rules! mark {
    ($owner:expr, $stage:ident) => {
        #[cfg(test)]
        $owner
            .trace
            .mark(crate::surface_javascript_wire::diagnostics::Stage::$stage);
    };
}
struct Apartment(bool);
impl Apartment {
    fn new() -> Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result == RPC_E_CHANGED_MODE {
            Ok(Self(false))
        } else {
            result.ok()?;
            Ok(Self(true))
        }
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}
struct Canvas {
    images: images::Bitmaps,
    backing: Option<Backing>,
    size: [u32; 2],
    encoded: Option<String>,
}
struct Renderer {
    canvases: BTreeMap<String, Canvas>,
    images: images::Inputs,
    factory: ID2D1Factory,
    text: crate::surface_text::Engine,
    budget: Arc<crate::byte_budget::Budget>,
    // 最后释放 apartment，确保所有同线程 COM 资源已销毁。
    _apartment: Apartment,
}
impl Renderer {
    fn new() -> Result<Self> {
        let apartment = Apartment::new()?;
        Ok(Self {
            canvases: BTreeMap::new(),
            images: images::Inputs::default(),
            factory: unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? },
            text: crate::surface_text::Engine::new()?,
            budget: crate::byte_budget::Budget::new(canvas::TOTAL_BYTES),
            _apartment: apartment,
        })
    }
    fn create(
        &self,
        size: [u32; 2],
        #[cfg(test)] trace: &mut crate::surface_javascript_wire::diagnostics::Trace,
    ) -> io::Result<Canvas> {
        let count = canvas::bytes(size)?;
        let backing = if count == 0 {
            None
        } else {
            Some(Backing::new(
                &self.factory,
                size,
                &self.budget,
                #[cfg(test)]
                trace,
            )?)
        };
        Ok(Canvas {
            images: images::Bitmaps::new(),
            backing,
            size,
            encoded: None,
        })
    }
    fn submit(
        &mut self,
        batch: commands::Batch,
        #[cfg(test)] trace: &mut crate::surface_javascript_wire::diagnostics::Trace,
    ) -> io::Result<()> {
        batch.validate()?;
        if !self.canvases.contains_key(&batch.id) && self.canvases.len() >= canvas::MAX_CANVASES {
            return Err(invalid());
        }
        if self
            .canvases
            .get(&batch.id)
            .is_none_or(|c| c.size != batch.size)
        {
            self.canvases.remove(&batch.id);
            let canvas = self.create(
                batch.size,
                #[cfg(test)]
                trace,
            )?;
            self.canvases.insert(batch.id.clone(), canvas);
        }
        let canvas = self.canvases.get_mut(&batch.id).unwrap();
        if !batch.reset && batch.commands.is_empty() {
            return Ok(());
        }
        canvas.encoded = None;
        let Some(backing) = &canvas.backing else {
            return Ok(());
        };
        // 位图在BeginDraw前创建一次并由target owner持有，避免8192条命令积累临时副本。
        for command in &batch.commands {
            if let commands::Draw::Image { .. } = command {
                self.images
                    .prepare(&backing.target, &mut canvas.images, command, &self.budget)?;
            }
        }
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasImagesReady);
        unsafe {
            backing.target.BeginDraw();
        }
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasCommandsBegin);
        let result = (|| {
            if batch.reset {
                draw::draw(
                    &backing.target,
                    &self.factory,
                    &self.text,
                    &canvas.images,
                    &commands::Draw::Clear {
                        rect: [0., 0., batch.size[0] as f32, batch.size[1] as f32],
                        transform: [1., 0., 0., 1., 0., 0.],
                    },
                )?;
            }
            for command in &batch.commands {
                draw::draw(
                    &backing.target,
                    &self.factory,
                    &self.text,
                    &canvas.images,
                    command,
                )?;
            }
            Ok::<_, windows::core::Error>(())
        })();
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasCommandsReady);
        let ended = unsafe { backing.target.EndDraw(None, None) };
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasEndDrawReady);
        result.and(ended).map_err(|_| invalid())
    }
    fn snapshot(&mut self, frame: &mut canvas::CanvasFrame) -> io::Result<()> {
        let owner = self.canvases.get_mut(&frame.id).ok_or_else(invalid)?;
        if owner.size != [frame.width, frame.height] {
            return Err(invalid());
        }
        if owner.encoded.is_none() {
            let encoded = if let Some(backing) = &owner.backing {
                let mut pixels = vec![0; canvas::bytes(owner.size)?];
                backing.copy_into(&mut pixels)?;
                canvas::encode(owner.size, pixels)?
            } else {
                String::new()
            };
            owner.encoded = Some(encoded);
        }
        frame.png = owner.encoded.as_ref().unwrap().clone();
        Ok(())
    }
}
#[derive(Default)]
pub(super) struct Store {
    renderer: Option<Renderer>,
    deadline: Option<Instant>,
    failed: bool,
    commands: usize,
    submissions: usize,
    #[cfg(test)]
    pub(super) trace: crate::surface_javascript_wire::diagnostics::Trace,
}
impl Store {
    pub(super) fn install_images(
        &mut self,
        headers: &[crate::surface_javascript_wire::images::Header],
        reader: &mut impl io::Read,
    ) -> io::Result<()> {
        if self.renderer.is_some() || self.failed {
            return Err(invalid());
        }
        let mut renderer = Renderer::new().map_err(|_| invalid())?;
        renderer.images.install(headers, reader, &renderer.budget)?;
        self.renderer = Some(renderer);
        Ok(())
    }
    pub(super) fn image_size(&self, id: &str) -> Option<[u32; 2]> {
        self.renderer.as_ref()?.images.size(id)
    }
    pub(super) fn append_image(
        &mut self,
        header: &crate::surface_javascript_wire::images::Header,
        png: &crate::pixels::Pixels,
    ) -> io::Result<()> {
        if self.failed {
            return Err(invalid());
        }
        if self.renderer.is_none() {
            self.renderer = Some(Renderer::new().map_err(|_| invalid())?);
        }
        let renderer = self.renderer.as_mut().unwrap();
        renderer.images.install(
            std::slice::from_ref(header),
            &mut io::Cursor::new(&**png),
            &renderer.budget,
        )
    }
    pub(super) fn begin(&mut self, deadline: Instant) {
        #[cfg(test)]
        self.trace.reset();
        mark!(self, CanvasBegin);
        self.deadline = Some(deadline);
        self.commands = 0;
        self.submissions = 0;
    }
    pub(super) fn valid(&self) -> bool {
        !self.failed && self.deadline.is_some_and(|d| Instant::now() < d)
    }
    // 保留本 operation 已消耗的绘制/提交预算，等待不等于新 begin。
    pub(super) fn exclude_wait(&mut self, elapsed: std::time::Duration) {
        self.deadline = self.deadline.and_then(|v| v.checked_add(elapsed));
    }
    pub(super) fn submit(&mut self, source: &str) -> bool {
        mark!(self, CanvasAdmission);
        let result = (|| {
            self.submissions += 1;
            if !self.valid() || source.len() > 1024 * 1024 || self.submissions > 8 {
                return Err(invalid());
            }
            mark!(self, CanvasParse);
            let batch: commands::Batch = serde_json::from_str(source).map_err(|_| invalid())?;
            self.commands += batch.commands.len();
            if self.commands > 8192 {
                return Err(invalid());
            }
            if self.renderer.is_none() {
                mark!(self, CanvasRenderer);
                self.renderer = Some(Renderer::new().map_err(|_| invalid())?);
                mark!(self, CanvasRendererReady);
            }
            mark!(self, CanvasDraw);
            let result = self.renderer.as_mut().unwrap().submit(
                batch,
                #[cfg(test)]
                &mut self.trace,
            );
            mark!(self, CanvasDrawReturned);
            result
        })();
        if result.is_err() || !self.valid() {
            mark!(self, CanvasSubmitFailed);
            self.failed = true;
            false
        } else {
            mark!(self, CanvasSubmitComplete);
            true
        }
    }
    pub(super) fn snapshots(&mut self, frames: &mut [canvas::CanvasFrame]) -> io::Result<()> {
        mark!(self, CanvasSnapshotAdmission);
        let result = self.collect_snapshots(frames);
        if result.is_err() {
            mark!(self, CanvasSnapshotFailed);
            self.failed = true;
        }
        result
    }
    fn collect_snapshots(&mut self, frames: &mut [canvas::CanvasFrame]) -> io::Result<()> {
        if frames.len() > canvas::MAX_CANVASES || !self.valid() {
            return Err(invalid());
        }
        let mut encoded = 0;
        for frame in frames {
            mark!(self, CanvasSnapshot);
            self.renderer
                .as_mut()
                .ok_or_else(invalid)?
                .snapshot(frame)?;
            mark!(self, CanvasSnapshotComplete);
            encoded += frame.png.len();
            if encoded > canvas::MAX_ENCODED {
                return Err(invalid());
            }
        }
        if !self.valid() {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn dispose(&mut self) {
        self.renderer.take();
    }
    #[cfg(test)]
    pub(super) fn diagnostic_failed(&self) -> bool {
        self.failed
    }
    #[cfg(test)]
    pub(super) fn test_budget(&self) -> Arc<crate::byte_budget::Budget> {
        self.renderer.as_ref().unwrap().budget.clone()
    }
}
