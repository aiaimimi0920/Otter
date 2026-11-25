//! Actual native declarative host capabilities shared by attach and explicit restore.
use crate::surface_host_capabilities::{Input, Report, Runtime};

pub(crate) fn report() -> Result<Report, String> {
    Report::new(
        vec![Runtime::Declarative],
        [
            "view", "row", "column", "stack", "text", "spacer", "divider", "progress", "button",
            "icon", "input", "textarea", "number", "switch", "select", "slider", "image",
        ]
        .map(String::from)
        .to_vec(),
        // Bounded static images and original-lease transport are connected; the report itself grants no source-read permission.
        vec!["loom_resource".into()],
        vec!["remote_resources".into()],
        Input {
            pointer: true,
            hover: true,
            touch: false,
            keyboard: true,
        },
    )
    .map_err(|e| format!("宿主能力无效：{e:?}"))
}
