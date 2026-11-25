//! Public port names and baseline type icons; no parameter values, credentials, or images.
use crate::loom_capability::Capability;
use neuro_mini_ui::Icon;
pub(in crate::desktop) struct Presentation {
    pub label: String,
    pub icon: Icon,
}
fn icon(value_type: Option<&str>, widget: Option<&str>) -> Icon {
    let kind = value_type.unwrap_or("").to_lowercase();
    let widget = widget.unwrap_or("");
    macro_rules! svg {
        ($name:literal) => {
            Icon::svg(include_str!(concat!(
                "../../assets/edit-strip/Port",
                $name,
                ".svg"
            )))
        };
    }
    if matches!(widget, "select" | "radio") {
        svg!("Select")
    } else if widget == "color" || kind == "color" {
        svg!("Color")
    } else if matches!(widget, "file" | "image_link")
        || kind.contains("image")
        || kind.contains("texture")
    {
        svg!("Image")
    } else if matches!(widget, "checkbox" | "switch") || matches!(kind.as_str(), "bool" | "boolean")
    {
        svg!("Boolean")
    } else if matches!(widget, "slider" | "number")
        || matches!(
            kind.as_str(),
            "int" | "integer" | "float" | "double" | "number"
        )
    {
        svg!("Number")
    } else if matches!(widget, "text" | "textarea" | "path")
        || matches!(kind.as_str(), "str" | "string" | "text" | "path")
    {
        svg!("Text")
    } else {
        svg!("Data")
    }
}
impl Presentation {
    pub fn capture(
        capability: Option<&Capability<'_>>,
        name: &str,
        input: bool,
        image: bool,
    ) -> Self {
        let mut label = name;
        let mut value_type = image.then_some("image");
        let mut widget = None;
        let mut parameter_widget = None;
        if let Some(capability) = capability {
            let port = if input {
                capability.inputs().find(|p| p.name == name)
            } else {
                capability.outputs().find(|p| p.name == name)
            };
            if let Some(port) = port {
                label = port.label;
                value_type = Some(port.kind);
                widget = port.widget;
            }
            if input {
                if let Some(parameter) = capability.parameters().find(|p| p.id == name && !p.secret)
                {
                    label = parameter.label;
                    value_type = None; // Baseline paramPropertyType uses normalized widgets.
                    parameter_widget = Some(parameter.widget);
                }
            }
        }
        Self {
            label: label
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .take(128)
                .collect(),
            icon: icon(value_type, parameter_widget.as_deref().or(widget)),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_precedence_and_all_seven_shapes_are_supported() {
        for (kind, widget) in [
            ("number", "select"),
            ("color", ""),
            ("texture", ""),
            ("boolean", ""),
            ("float", ""),
            ("string", ""),
            ("object", ""),
        ] {
            icon(Some(kind), Some(widget)).validate().unwrap();
        }
        assert_eq!(
            icon(Some("number"), Some("radio")),
            icon(None, Some("select"))
        );
        assert_eq!(
            icon(Some("boolean"), Some("file")),
            icon(Some("image"), None)
        );
        assert_ne!(icon(Some("number"), None), icon(Some("string"), None));
    }
}
