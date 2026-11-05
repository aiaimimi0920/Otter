//! 隐藏 HWND 的一次准入；JS 权威 scene 不得替代后台执行后的 DOM 投影。
use super::*;
impl AuthorizedWindow {
    pub fn new(
        outcome: &Outcome,
        lease: Lease,
        now_ms: u64,
        size: [u32; 2],
    ) -> std::result::Result<Self, OpenError> {
        let (scene, snapshot_revision, resources, runtime) = outcome
            .snapshot_for(&lease, now_ms)
            .map_err(OpenError::Admission)?
            .into_parts();
        let notice = lease
            .revocation_notification()
            .map_err(OpenError::Notification)?;
        let scene = scene.with_image_availability(|_| false);
        let window = SurfaceWindow::new(scene, size[0], size[1]).map_err(OpenError::Window)?;
        if runtime == crate::surface_host_capabilities::Runtime::Javascript {
            window.enable_javascript_keyboard();
        }
        lease
            .validate(&outcome.origin, now_ms)
            .map_err(|e| OpenError::Admission(AdmissionError::Authorization(e)))?;
        let lifecycle = Owner::new(
            outcome
                .binding
                .as_ref()
                .expect("admission requires binding")
                .clone(),
        );
        Ok(Self {
            window,
            lease,
            notice,
            lifecycle: crate::surface_activation::Activation::new(lifecycle),
            stopped: None,
            request: outcome.request.clone(),
            activation_error: None,
            rejected_owner: None,
            shown: false,
            snapshot_revision,
            result_binding_pending: true,
            deferred_click: None,
            resources: std::cell::RefCell::new(Some(resources)),
            images: Default::default(),
            image_requests: Default::default(),
            javascript: (runtime == crate::surface_host_capabilities::Runtime::Javascript)
                .then(javascript::Execution::default),
        })
    }
}
