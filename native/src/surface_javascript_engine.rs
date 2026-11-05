//! 只由受限 child 调用的真实 ES module 引擎；每步墙钟/heap/stack 与生命周期串行。
use crate::surface_javascript_wire::{self as wire, Command, Output};
use rquickjs::{Context, Function, Module, Object, Persistent, Promise, Runtime};
use std::{
    cell::RefCell,
    io,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
#[path = "surface_javascript_canvas_raster.rs"]
mod canvas;
#[path = "surface_javascript_geometry.rs"]
mod geometry;
#[path = "surface_javascript_engine_services.rs"]
mod services;
macro_rules! mark {
    ($owner:expr, $stage:ident) => {
        #[cfg(test)]
        $owner.trace.mark(wire::diagnostics::Stage::$stage);
    };
}
// 工厂只以参数传入私有闭包；不在 globalThis 暴露 state/nodes 或宿主桥。
const BOOTSTRAP: &str = concat!(
    "(",
    include_str!("surface_javascript_bootstrap.js"),
    ")(",
    include_str!("surface_javascript_tree.js"),
    ",",
    include_str!("surface_javascript_attributes.js"),
    ",",
    include_str!("surface_javascript_selectors.js"),
    ",",
    include_str!("surface_javascript_markup.js"),
    ",",
    include_str!("surface_javascript_json.js"),
    ",",
    include_str!("surface_javascript_css_grid_tracks.js"),
    ",",
    include_str!("surface_javascript_css_grid.js"),
    ",",
    include_str!("surface_javascript_css_spatial.js"),
    ",",
    include_str!("surface_javascript_css_values.js"),
    ",",
    include_str!("surface_javascript_css_selectors.js"),
    ",",
    include_str!("surface_javascript_css_declarations.js"),
    ",",
    include_str!("surface_javascript_css_sheets.js"),
    ",",
    include_str!("surface_javascript_css_projection.js"),
    ",",
    include_str!("surface_javascript_geometry.js"),
    ",",
    include_str!("surface_javascript_animation.js"),
    ",",
    include_str!("surface_javascript_timers.js"),
    ",",
    include_str!("surface_javascript_resize.js"),
    ",",
    include_str!("surface_javascript_canvas.js"),
    ",",
    include_str!("surface_javascript_canvas_context.js"),
    ",",
    include_str!("surface_javascript_canvas_path.js"),
    ",",
    include_str!("surface_javascript_css_fonts.js"),
    ",",
    include_str!("surface_javascript_event_target.js"),
    ",",
    include_str!("surface_javascript_form.js"),
    ",",
    include_str!("surface_javascript_focus.js"),
    ",",
    include_str!("surface_javascript_hover.js"),
    ",",
    include_str!("surface_javascript_css_borders.js"),
    ",",
    include_str!("surface_javascript_services.js"),
    ",",
    include_str!("surface_javascript_result.js"),
    ",",
    include_str!("surface_javascript_images.js"),
    ")"
);
pub(super) struct Engine {
    // Persistent/Context 必须早于 Runtime 释放，不能把 JS 值带出此线程或进程。
    bridge: Persistent<Object<'static>>,
    context: Context,
    runtime: Runtime,
    origin: Instant,
    deadline: Arc<AtomicU64>,
    mounted: bool,
    geometry: Rc<RefCell<geometry::Geometry>>,
    canvas: Rc<RefCell<canvas::Store>>,
    #[cfg(test)]
    trace: wire::diagnostics::Trace,
}
impl Engine {
    pub(super) fn new() -> io::Result<Self> {
        let runtime = Runtime::new().map_err(|_| wire::invalid())?;
        runtime.set_memory_limit(32 * 1024 * 1024);
        runtime.set_max_stack_size(256 * 1024);
        let origin = Instant::now();
        let deadline = Arc::new(AtomicU64::new(250_000));
        let clock = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            origin.elapsed().as_micros() >= u128::from(clock.load(Ordering::Relaxed))
        })));
        let context = Context::full(&runtime).map_err(|_| wire::invalid())?;
        let geometry = Rc::new(RefCell::new(geometry::Geometry::default()));
        let measure = geometry.clone();
        let canvas = Rc::new(RefCell::new(canvas::Store::default()));
        let raster = canvas.clone();
        let images = canvas.clone();
        let bridge = context
            .with(|ctx| {
                let function: Object = ctx.eval(BOOTSTRAP)?;
                let utf8_length = Function::new(ctx.clone(), |source: String| source.len() as u32)?;
                function
                    .get::<_, Function>("bindUtf8Length")?
                    .call::<_, ()>((utf8_length,))?;
                let native = Function::new(ctx.clone(), move |source: String, target: String| {
                    measure.borrow_mut().read(&source, &target)
                })?;
                function
                    .get::<_, Function>("bindGeometry")?
                    .call::<_, ()>((native,))?;
                let clock =
                    Function::new(ctx.clone(), move || origin.elapsed().as_secs_f64() * 1000.)?;
                function
                    .get::<_, Function>("bindAnimationClock")?
                    .call::<_, ()>((clock,))?;
                let submit = Function::new(ctx.clone(), move |source: String| {
                    raster.borrow_mut().submit(&source)
                })?;
                function
                    .get::<_, Function>("bindCanvas")?
                    .call::<_, ()>((submit,))?;
                let image = Function::new(ctx.clone(), move |id: String| {
                    images
                        .borrow()
                        .image_size(&id)
                        .map(|size| format!("[{},{}]", size[0], size[1]))
                        .unwrap_or_default()
                })?;
                function
                    .get::<_, Function>("bindImages")?
                    .call::<_, ()>((image,))?;
                Ok::<_, rquickjs::Error>(Persistent::save(&ctx, function))
            })
            .map_err(|_| wire::invalid())?;
        Ok(Self {
            bridge,
            context,
            runtime,
            origin,
            deadline,
            mounted: false,
            geometry,
            canvas,
            #[cfg(test)]
            trace: wire::diagnostics::Trace::default(),
        })
    }
    pub(super) fn install_images(
        &mut self,
        headers: &[wire::images::Header],
        reader: &mut impl io::Read,
    ) -> io::Result<()> {
        if self.mounted {
            return Err(wire::invalid());
        }
        self.canvas.borrow_mut().install_images(headers, reader)
    }
    #[cfg(test)]
    pub(super) fn apply(
        &mut self,
        command: Command,
        viewport: Option<crate::surface_viewport::Viewport>,
    ) -> io::Result<Output> {
        self.apply_services(command, viewport, &mut |_| Err(wire::invalid()))
    }
    pub(super) fn apply_services(
        &mut self,
        command: Command,
        viewport: Option<crate::surface_viewport::Viewport>,
        dispatch: &mut dyn FnMut(wire::service::Call) -> io::Result<wire::service::ResultValue>,
    ) -> io::Result<Output> {
        let animating = matches!(&command, Command::AnimationFrame);
        let ticking = animating || matches!(&command, Command::TimerTick);
        let disposing = matches!(&command, Command::Dispose);
        #[cfg(test)]
        self.trace.reset();
        mark!(self, EngineBegin);
        self.canvas
            .borrow_mut()
            .begin(Instant::now() + std::time::Duration::from_millis(250));
        mark!(self, GeometryBegin);
        self.geometry.borrow_mut().begin(
            viewport,
            Instant::now() + std::time::Duration::from_millis(250),
        )?;
        self.deadline.store(
            (self.origin.elapsed().as_micros() as u64).saturating_add(250_000),
            Ordering::Relaxed,
        );
        mark!(self, BridgeBegin);
        self.context
            .with(|ctx| {
                let bridge = self.bridge.clone().restore(&ctx)?;
                let begin: Function = bridge.get("beginGeometry")?;
                bridge
                    .get::<_, Function>("beginCanvas")?
                    .call::<_, ()>(())?;
                begin.call::<_, ()>((serde_json::to_string(&viewport.map(|v| v.ratio()))
                    .map_err(|_| rquickjs::Error::Unknown)?,))
            })
            .map_err(|_| wire::invalid())?;
        let command = match command {
            Command::InstallImages { .. } => return Err(wire::invalid()),
            Command::Mount {
                source,
                snapshot,
                resources,
            } => {
                if self.mounted
                    || source.is_empty()
                    || source.len() > wire::MAX_SOURCE
                    || source.contains('\0')
                {
                    return Err(wire::invalid());
                }
                self.mounted = true;
                mark!(self, Module);
                self.context
                    .with(|ctx| Module::evaluate(ctx, "surface:entry", source)?.finish::<()>())
                    .map_err(|_| wire::invalid())?;
                Command::Mount {
                    source: String::new(),
                    snapshot,
                    resources,
                }
            }
            Command::LocalInput { input } if self.mounted => {
                input.validate()?;
                Command::LocalInput { input }
            }
            Command::LiveTrigger { event } if self.mounted => {
                if !event.validate() {
                    return Err(wire::invalid());
                }
                Command::LiveTrigger { event }
            }
            value if self.mounted => value,
            _ => return Err(wire::invalid()),
        };
        let bytes = wire::encode(&command)?;
        let json = std::str::from_utf8(&bytes).map_err(|_| wire::invalid())?;
        mark!(self, Apply);
        let pending = self
            .context
            .with(|ctx| {
                let bridge = self.bridge.clone().restore(&ctx)?;
                let function: Function = bridge.get("apply")?;
                let pending: Promise = function.call((json,))?;
                Ok::<_, rquickjs::Error>(Persistent::save(&ctx, pending))
            })
            .map_err(|_| wire::invalid())?;
        self.settle_services(Some(pending), dispatch)?;
        mark!(self, Microtasks);
        self.drain_jobs()?;
        if ticking {
            mark!(self, Timers);
            self.callbacks("timerStep", 64)?;
        }
        if animating {
            mark!(self, Animation);
            self.callbacks("animationStep", 64)?;
        }
        mark!(self, ResizeBegin);
        self.context
            .with(|ctx| {
                self.bridge
                    .clone()
                    .restore(&ctx)?
                    .get::<_, Function>("beginResize")?
                    .call::<_, ()>(())
            })
            .map_err(|_| wire::invalid())?;
        mark!(self, Resize);
        self.callbacks("resizeStep", 256)?;
        self.settle_services(None, dispatch)?;
        self.context
            .with(|ctx| {
                self.bridge
                    .clone()
                    .restore(&ctx)?
                    .get::<_, Function>("endServices")?
                    .call::<_, ()>(())
            })
            .map_err(|_| wire::invalid())?;
        mark!(self, Collect);
        let result: String = self
            .context
            .with(|ctx| {
                let bridge = self.bridge.clone().restore(&ctx)?;
                let collect: Function = bridge.get("collect")?;
                collect.call::<_, String>(())
            })
            .map_err(|_| wire::invalid())?;
        if result.len() > wire::MAX_FRAME || !self.geometry.borrow().valid() {
            return Err(wire::invalid());
        }
        mark!(self, OutputParse);
        let mut output: Output = serde_json::from_str(&result).map_err(|_| wire::invalid())?;
        mark!(self, CanvasSnapshots);
        self.canvas.borrow_mut().snapshots(&mut output.canvases)?;
        if disposing {
            self.canvas.borrow_mut().dispose();
        }
        // PNG 编码也是本 operation 的原 250ms 总期限，不另开宽限窗口。
        mark!(self, FinalValidation);
        if !self.geometry.borrow().valid() || !self.canvas.borrow().valid() {
            return Err(wire::invalid());
        }
        mark!(self, Encode);
        wire::encode(&output)?;
        mark!(self, Complete);
        Ok(output)
    }
    #[cfg(test)]
    pub(super) fn diagnostic(&self) -> wire::diagnostics::Report {
        let canvas = self.canvas.borrow();
        wire::diagnostics::Report {
            engine: self.trace.samples(),
            canvas: canvas.trace.samples(),
            engine_expired: self.origin.elapsed().as_micros()
                >= u128::from(self.deadline.load(Ordering::Relaxed)),
            geometry_valid: self.geometry.borrow().valid(),
            canvas_valid: canvas.valid(),
            canvas_failed: canvas.diagnostic_failed(),
        }
    }
    fn callbacks(&self, method: &str, maximum: usize) -> io::Result<()> {
        // rAF/observer 都在原总期限内；每次 callback 之后完成独立 microtask checkpoint。
        for _ in 0..=maximum {
            let ran = self
                .context
                .with(|ctx| {
                    self.bridge
                        .clone()
                        .restore(&ctx)?
                        .get::<_, Function>(method)?
                        .call::<_, bool>(())
                })
                .map_err(|_| wire::invalid())?;
            if !ran {
                return Ok(());
            }
            self.drain_jobs()?;
        }
        Err(wire::invalid())
    }
    fn drain_jobs(&self) -> io::Result<()> {
        // 不遗留 detached Promise jobs；自续 microtask 与全部同帧 callback 共用原总期限。
        while self.runtime.is_job_pending() {
            if self.origin.elapsed().as_micros()
                >= u128::from(self.deadline.load(Ordering::Relaxed))
            {
                return Err(wire::invalid());
            }
            self.runtime
                .execute_pending_job()
                .map_err(|_| wire::invalid())?;
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_javascript_animation_model_tests.rs"]
mod animation_model_tests;
#[cfg(test)]
#[path = "surface_javascript_border_model_tests.rs"]
mod border_model_tests;
#[cfg(test)]
#[path = "surface_javascript_canvas_image_tests.rs"]
mod canvas_image_tests;
#[cfg(test)]
#[path = "surface_javascript_canvas_model_tests.rs"]
mod canvas_model_tests;
#[cfg(test)]
#[path = "surface_javascript_canvas_service_model_tests.rs"]
mod canvas_service_tests;
#[cfg(test)]
#[path = "surface_javascript_cascade_record_tests.rs"]
mod cascade_record_tests;
#[cfg(test)]
#[path = "surface_javascript_css_budget_tests.rs"]
mod css_budget_tests;
#[cfg(test)]
#[path = "surface_javascript_css_hover_model_tests.rs"]
mod css_hover_model_tests;
#[cfg(test)]
#[path = "surface_javascript_css_model_tests.rs"]
mod css_model_tests;
#[cfg(test)]
#[path = "surface_javascript_css_phase_tests.rs"]
mod css_phase_tests;
#[cfg(test)]
#[path = "surface_javascript_css_projection_tests.rs"]
mod css_projection_tests;
#[cfg(test)]
#[path = "surface_javascript_diagnostic_tests.rs"]
mod diagnostic_tests;
#[cfg(test)]
#[path = "surface_javascript_document_model_tests.rs"]
mod document_model_tests;
#[cfg(test)]
#[path = "surface_javascript_event_model_tests.rs"]
mod event_model_tests;
#[cfg(test)]
#[path = "surface_javascript_form_budget_tests.rs"]
mod form_budget_tests;
#[cfg(test)]
#[path = "surface_javascript_form_value_model_tests.rs"]
mod form_value_model_tests;
#[cfg(test)]
#[path = "surface_javascript_geometry_projection_tests.rs"]
mod geometry_projection_tests;
#[cfg(test)]
#[path = "surface_javascript_geometry_engine_tests.rs"]
mod geometry_tests;
#[cfg(test)]
#[path = "surface_javascript_grid_model_tests.rs"]
mod grid_model_tests;
#[cfg(test)]
#[path = "surface_javascript_json_model_tests.rs"]
mod json_model_tests;
#[cfg(test)]
#[path = "surface_javascript_nth_model_tests.rs"]
mod nth_model_tests;
#[cfg(test)]
#[path = "surface_javascript_padding_model_tests.rs"]
mod padding_model_tests;
#[cfg(test)]
#[path = "surface_javascript_pointer_model_tests.rs"]
mod pointer_model_tests;
#[cfg(test)]
#[path = "surface_javascript_resize_model_tests.rs"]
mod resize_model_tests;
#[cfg(test)]
#[path = "surface_javascript_result_model_tests.rs"]
mod result_tests;
#[cfg(test)]
#[path = "surface_javascript_service_image_model_tests.rs"]
mod service_image_tests;
#[cfg(test)]
#[path = "surface_javascript_engine_service_tests.rs"]
mod service_tests;
#[cfg(test)]
#[path = "surface_javascript_spatial_model_tests.rs"]
mod spatial_model_tests;
#[cfg(test)]
#[path = "surface_javascript_text_overflow_model_tests.rs"]
mod text_overflow_model_tests;
#[cfg(test)]
#[path = "surface_javascript_timer_model_tests.rs"]
mod timer_model_tests;
#[cfg(test)]
#[path = "surface_javascript_typography_model_tests.rs"]
mod typography_model_tests;
