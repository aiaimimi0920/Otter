//! 声明式文字场景的 HWND 后端；窗口 owner 负责消息循环和销毁顺序。
use crate::{
    surface_presentation::{Images, Presentation},
    surface_scene::Scene,
    surface_viewport::{context, style, Viewport},
};
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Direct2D::{Common::*, *},
    },
};
#[path = "surface_host_accessibility.rs"]
mod accessibility;
#[path = "surface_host_edit.rs"]
mod edit;
#[path = "surface_host_pointer.rs"]
mod pointer;
#[path = "surface_host_scrollbar.rs"]
mod scrollbar;
#[path = "surface_host_selection.rs"]
mod selection;
pub struct Host {
    hwnd: HWND,
    factory: ID2D1Factory,
    target: Option<(ID2D1HwndRenderTarget, ID2D1SolidColorBrush)>,
    presentation: Presentation,
    viewport: (D2D_SIZE_U, u32),
    edits: super::surface_edit_mount::Mount,
    // HWND 与单线程 D2D 工厂不得转移到工作线程。
    _thread: PhantomData<Rc<()>>,
}
fn logical(size: D2D_SIZE_U, dpi: u32) -> [f32; 2] {
    Viewport::new([size.width, size.height], dpi)
        .expect("validated HWND viewport")
        .logical()
}
fn viewport(hwnd: HWND) -> Result<(D2D_SIZE_U, u32)> {
    let view = Viewport::from_window(hwnd)?;
    Ok((
        D2D_SIZE_U {
            width: view.pixels()[0],
            height: view.pixels()[1],
        },
        view.dpi(),
    ))
}
impl Host {
    /// 调用方必须在 HWND 所属线程使用，并在窗口销毁前释放 Host。
    pub fn bind(hwnd: HWND, scene: Scene) -> Result<Self> {
        Self::bind_with_images(hwnd, scene, Images::new())
    }
    /// 图片必须已由调用方核对当前场景来源；此层只拥有布局、像素与 target 生命周期。
    pub fn bind_with_images(hwnd: HWND, scene: Scene, images: Images) -> Result<Self> {
        if unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, None) }
            != unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Surface HWND belongs to another thread",
            ));
        }
        let view = viewport(hwnd)?;
        let size = logical(view.0, view.1);
        let presentation = Presentation::with_images(scene, style(), context(size), size, images)?;
        let factory = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        Ok(Self {
            hwnd,
            factory,
            target: None,
            presentation,
            viewport: view,
            edits: Default::default(),
            _thread: PhantomData,
        })
    }
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }
    pub(crate) fn scroll(
        &mut self,
        pixels: [i32; 2],
        axis: usize,
        delta: f32,
        page: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        let changed = self.presentation.wheel(point, axis, delta, page)?;
        if changed {
            self.sync_edits()?;
        }
        Ok(changed)
    }
    pub fn replace(&mut self, scene: Scene) -> Result<()> {
        self.replace_with_images(scene, Images::new())
    }
    pub fn replace_with_images(&mut self, scene: Scene, images: Images) -> Result<()> {
        self.replace_image_scene(scene, images, false)
    }
    pub(super) fn refresh_images(&mut self, scene: Scene, images: Images) -> Result<()> {
        self.replace_image_scene(scene, images, true)
    }
    fn replace_image_scene(
        &mut self,
        scene: Scene,
        images: Images,
        same_scene: bool,
    ) -> Result<()> {
        let view = viewport(self.hwnd)?;
        let size = logical(view.0, view.1);
        if same_scene {
            self.presentation
                .refresh_images(scene, style(), context(size), size, images)?;
        } else {
            self.presentation
                .replace_with_images(scene, style(), context(size), size, images)?;
        }
        if view.0.width != self.viewport.0.width
            || view.0.height != self.viewport.0.height
            || view.1 != self.viewport.1
        {
            self.presentation.release_device_resources();
            self.target = None;
        }
        self.viewport = view;
        self.sync_edits()
    }
    /// 同步几何但不创建渲染 target；命中不能等待一次 WM_PAINT 才看见新视口。
    fn sync_viewport(&mut self) -> Result<bool> {
        let view = viewport(self.hwnd)?;
        if view.0.width == 0 || view.0.height == 0 {
            return Ok(false);
        }
        if view.0.width != self.viewport.0.width
            || view.0.height != self.viewport.0.height
            || view.1 != self.viewport.1
        {
            let size = logical(view.0, view.1);
            self.presentation.reflow(style(), context(size), size)?;
            let previous = self.viewport;
            self.viewport = view;
            self.presentation.release_device_resources();
            self.target = None;
            if let Err(error) = self.sync_edits() {
                // 不缓存失败的控件布局，否则下一次输入会跳过准入并使用部分更新几何。
                self.viewport = previous;
                return Err(error);
            }
        }
        Ok(true)
    }
    pub fn hit_test(&mut self, client_pixels: [i32; 2]) -> Result<Option<&str>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        let scale = 96. / self.viewport.1 as f32;
        Ok(self
            .presentation
            .hit_test(client_pixels.map(|value| value as f32 * scale)))
    }
    pub(super) fn button_enabled_at(&mut self, client_pixels: [i32; 2]) -> Result<Option<bool>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        let scale = 96. / self.viewport.1 as f32;
        Ok(self
            .presentation
            .button_enabled_at(client_pixels.map(|value| value as f32 * scale)))
    }
    pub fn click_events(
        &mut self,
        client_pixels: [i32; 2],
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        if !self.sync_viewport()? {
            return Ok(Vec::new());
        }
        let scale = 96. / self.viewport.1 as f32;
        self.presentation
            .click_events_with_values(
                client_pixels.map(|value| value as f32 * scale),
                binding,
                revision,
                modifiers,
                self.edits.values(),
            )
            .map_err(|_| Error::new(E_INVALIDARG, "Surface click preparation failed"))
    }
    #[cfg(test)]
    pub(super) fn keyboard_events(
        &mut self,
        id: &str,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        if !self.sync_viewport()? {
            return Ok(Vec::new());
        }
        self.presentation
            .keyboard_events_with_values(id, binding, revision, modifiers, self.edits.values())
            .map_err(|_| Error::new(E_INVALIDARG, "Surface keyboard preparation failed"))
    }
    /// 焦点选择与点击一样先同步视口，不使用已过期的布局快照。
    pub(super) fn next_focus(
        &mut self,
        current: Option<&str>,
        reverse: bool,
    ) -> Result<Option<String>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        let next = self.presentation.reveal_next_focus(current, reverse)?;
        self.sync_edits()?;
        Ok(next)
    }
    pub(super) fn focusable(&mut self, id: &str) -> Result<bool> {
        Ok(self.sync_viewport()? && self.presentation.focusable(id))
    }
    /// WM_PAINT 调用方管理 BeginPaint/EndPaint；此处不启动定时器或循环重绘。
    pub fn paint(&mut self) -> Result<()> {
        self.paint_with_visual(Default::default())
    }
    pub(super) fn paint_with_visual(
        &mut self,
        visual: crate::surface_button_visual::State<'_>,
    ) -> Result<()> {
        if !self.sync_viewport()? {
            return Ok(());
        }
        let view = self.viewport;
        if self.target.is_none() {
            self.target = Some(unsafe {
                let target = self.factory.CreateHwndRenderTarget(
                    &D2D1_RENDER_TARGET_PROPERTIES {
                        dpiX: view.1 as f32,
                        dpiY: view.1 as f32,
                        ..Default::default()
                    },
                    &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                        hwnd: self.hwnd,
                        pixelSize: view.0,
                        ..Default::default()
                    },
                )?;
                let brush = target.CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 1.,
                    },
                    None,
                )?;
                (target, brush)
            });
        }
        let (target, brush) = self.target.as_ref().unwrap();
        let result = unsafe {
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F {
                r: 1.,
                g: 1.,
                b: 1.,
                a: 1.,
            }));
            let draw = self
                .presentation
                .draw_with_switches(target, brush, visual, |id| self.edits.switch_value(id));
            let finish = target.EndDraw(None, None);
            draw.and(finish)
        };
        // 设备错误后释放目标；下一次外部重绘请求再创建，避免故障忙循环。
        if result.is_err() {
            self.presentation.release_device_resources();
            self.target = None;
        }
        result
    }
}
#[cfg(test)]
#[path = "surface_host_tests.rs"]
mod tests;
