//! Immutable public configuration retained during asynchronous input preparation; holds no graph borrows, secrets, or image data.
use super::*;
pub struct Configuration {
    pub(super) art_id: String,
    pub(super) image_ports: Vec<String>,
    pub(super) tail: String,
}
impl Configuration {
    /// The graph layer has completed public-field filtering and connection conversion; do not reread UI state during encoding.
    pub fn from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, false, false)
    }
    /// Shader-code retrieval and local binding consume the same resolved public parameters; forced mode is unaffected by manual disabling.
    pub fn shader_from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, true, false)
    }
    /// The current host installation determines the ABI; scripts/connections/disabled parameters cannot override it.
    pub fn native_shader_from_resolved(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::resolved_mode(art_id, parameters, images, true, true)
    }
    fn resolved_mode(
        art_id: &str,
        parameters: &crate::graph_params::Params<'_>,
        images: &[&str],
        shader: bool,
        native: bool,
    ) -> Result<Self, Error> {
        let image_ports = ports(art_id, images.iter().copied())?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        if shader {
            // Loom merges inputs/parameters; do not discard real images or override them with a same-name forced mode.
            if image_ports.iter().any(|name| {
                matches!(name.as_str(), "mode" | "output_mode") || (native && name == "shader_abi")
            }) {
                return Err(Error::InvalidValue);
            }
            json.raw("\"output_mode\":\"shader\",\"mode\":\"shader\"")?;
            if native {
                json.raw(",\"shader_abi\":\"otter.hlsl.v1\"")?;
            }
            first = false;
        }
        for (name, value) in parameters.entries() {
            // Loom merges inputs and parameters into one parameter object; image sources replace same-name scalars.
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            if (shader && matches!(name, "mode" | "output_mode"))
                || (native && name == "shader_abi")
            {
                continue;
            }
            let Some(value) = value else { continue };
            if matches!(
                value,
                Output::Other | Output::Image(crate::graph_params::DISABLED)
            ) {
                continue;
            }
            json.field(name, &mut first)?;
            json.value(value, 0)?;
        }
        finish(
            art_id,
            image_ports,
            json,
            parameters.entries().filter_map(|(name, value)| {
                (value == Some(Output::Image(crate::graph_params::DISABLED))
                    && !(shader && matches!(name, "mode" | "output_mode"))
                    && !(native && name == "shader_abi"))
                    .then_some(name)
            }),
        )
    }
    pub fn capture(art_id: &str, prepared: &Prepared<'_>) -> Result<Self, Error> {
        let image_ports = ports(art_id, prepared.images.iter().map(|(name, _)| *name))?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        for (name, value) in prepared.active_params() {
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            let Some(value) = value else {
                continue;
            };
            if matches!(value, Output::Other) {
                continue;
            }
            json.field(name, &mut first)?;
            json.value(value, 0)?;
        }
        finish(art_id, image_ports, json, prepared.disabled.iter().copied())
    }
    /// Create an independent snapshot from filtered, bounded public editing state without bypassing Params' secret/internal-field boundaries.
    pub fn from_parameters(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::parameters_mode(parameters, images, false)
    }
    /// Still read only public Parameters; force Shader-prefetch mode and transmit images through formal inputs descriptors.
    pub fn shader_from_parameters(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
    ) -> Result<Self, Error> {
        Self::parameters_mode(parameters, images, true)
    }
    fn parameters_mode(
        parameters: &crate::loom_parameter_state::Parameters,
        images: &[&str],
        shader: bool,
    ) -> Result<Self, Error> {
        let image_ports = ports(parameters.art_id(), images.iter().copied())?;
        let mut json = Json::with_limit(crate::graph_execution_wire::MAX_MESSAGE_BYTES);
        json.raw(",\"parameters\":{")?;
        let mut first = true;
        if shader {
            // Use the same admission rules as resolved snapshots; ordinary Art still allows these two image-port names.
            if image_ports
                .iter()
                .any(|name| matches!(name.as_str(), "mode" | "output_mode"))
            {
                return Err(Error::InvalidValue);
            }
            json.raw("\"output_mode\":\"shader\",\"mode\":\"shader\"")?;
            first = false;
        }
        for (name, value) in parameters.entries() {
            if image_ports.iter().any(|port| port == name) {
                continue;
            }
            if shader && matches!(name, "output_mode" | "mode") {
                continue;
            }
            let Some(value) = value else {
                continue;
            };
            if disabled(value) {
                continue;
            }
            json.field(name, &mut first)?;
            let encoded = serde_json::to_string(value).map_err(|_| Error::InvalidValue)?;
            json.raw(&encoded)?;
        }
        finish(
            parameters.art_id(),
            image_ports,
            json,
            parameters.entries().filter_map(|(name, value)| {
                (value.is_some_and(disabled) && !(shader && matches!(name, "output_mode" | "mode")))
                    .then_some(name)
            }),
        )
    }
}
fn disabled(value: &serde_json::Value) -> bool {
    value.as_str() == Some(crate::graph_params::DISABLED)
}
#[cfg(test)]
#[path = "loom_art_config_tests.rs"]
mod tests;
fn ports<'a>(art_id: &str, names: impl Iterator<Item = &'a str>) -> Result<Vec<String>, Error> {
    if art_id.is_empty() || art_id.len() > 256 || art_id.chars().any(char::is_control) {
        return Err(Error::InvalidArtId);
    }
    let mut image_ports = Vec::new();
    for name in names {
        if image_ports.len() >= 32
            || name.is_empty()
            || name.len() > 256
            || name.chars().any(char::is_control)
            || image_ports.iter().any(|previous| previous == name)
        {
            return Err(Error::InvalidValue);
        }
        image_ports.try_reserve(1).map_err(|_| Error::Allocation)?;
        image_ports.push(name.to_owned());
    }
    Ok(image_ports)
}
fn finish<'a>(
    art_id: &str,
    image_ports: Vec<String>,
    mut json: Json,
    disabled: impl Iterator<Item = &'a str>,
) -> Result<Configuration, Error> {
    json.raw("},\"disabledParameters\":[")?;
    for (index, name) in disabled.enumerate() {
        if index != 0 {
            json.raw(",")?;
        }
        json.string(name)?;
    }
    json.raw("]")?;
    Ok(Configuration {
        art_id: art_id.into(),
        image_ports,
        tail: json.0,
    })
}
