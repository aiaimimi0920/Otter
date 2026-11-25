//! Explicit local Unit references in shared parameters; reuse the sole Graph/Source without granting path, network, or preview-read permission.
use crate::{
    graph_image::{Graph, Output},
    graph_params::Params,
    loom_capability::Capability,
    workspace::Workspace,
    workspace_id::UnitId,
};

pub(super) fn references<'a>(
    workspace: &'a Workspace,
    parameters: Option<&Params<'_>>,
    input: &str,
) -> Result<Vec<(String, &'a UnitId)>, &'static str> {
    let mut references = Vec::new();
    let Some(parameters) = parameters else {
        return Ok(references);
    };
    for (name, value) in parameters.entries() {
        if !crate::desktop::shader_parameters::public_texture_name(name, input) {
            continue;
        }
        let Some(Output::Image(value)) = value else {
            continue;
        };
        if value == crate::graph_params::DISABLED {
            continue;
        }
        // Match current workspace identity byte for byte; unknown strings/URLs/paths are not read requests.
        let Some(unit) = workspace.units().iter().find(|unit| unit.as_str() == value) else {
            continue;
        };
        crate::shader_angle::name(name)
            .map_err(|_| crate::ui_text::text("app-shader-input-name-invalid"))?;
        if references.len() >= 31 {
            return Err(crate::ui_text::text("app-shader-input-count-limit"));
        }
        references
            .try_reserve(1)
            .map_err(|_| crate::ui_text::text("app-shader-reference-allocation"))?;
        references.push((name.into(), unit));
    }
    Ok(references)
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "application_shader_input_tests.rs"]
mod tests;

pub(super) fn resolve<'a>(
    shader: Option<&Capability<'_>>,
    graph: &Graph<'a>,
    references: &'a [(String, &UnitId)],
    images: &mut Vec<(&'a str, &'a str)>,
) -> Result<Prepared<'a>, &'static str> {
    let mut prepared = Prepared::default();
    let ports: Vec<_> = images.iter().map(|(name, _)| *name).collect();
    let Some(input) = shader.and_then(|capability| capability.available_shader_input(&ports))
    else {
        return Ok(prepared);
    };
    for (name, unit) in references {
        if !crate::desktop::shader_parameters::public_texture_name(name, input) {
            continue;
        }
        if !images.iter().any(|(port, _)| *port == name) {
            // Art may use committed output only; Graph still handles Sticker relay/edit/Live sources.
            // Do not call image()/canvas_image() to obtain Art's own display or restored preview.
            let Some(Output::Image(key)) = graph.output(unit, "output").map(Output::direct) else {
                return Err(crate::ui_text::text("app-shader-reference-no-image"));
            };
            images
                .try_reserve(1)
                .map_err(|_| crate::ui_text::text("app-shader-image-allocation"))?;
            images.push((name.as_str(), key));
            prepared
                .units
                .try_reserve(1)
                .map_err(|_| crate::ui_text::text("app-shader-source-freeze"))?;
            prepared.units.push(*unit);
        }
        prepared
            .parameters
            .try_reserve(1)
            .map_err(|_| crate::ui_text::text("app-shader-image-freeze"))?;
        prepared.parameters.push(name.clone());
    }
    Ok(prepared)
}
#[derive(Default)]
pub(super) struct Prepared<'a> {
    pub parameters: Vec<String>,
    pub units: Vec<&'a UnitId>,
}
