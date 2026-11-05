//! 声明式场景的渲染前合同；只解析数据，不执行脚本、读取资源或宣称节点已可绘制。
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_NODES: usize = 2048;
pub const MAX_DEPTH: usize = 32;
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    View,
    Row,
    Column,
    Stack,
    Grid,
    Scroll,
    Text,
    Image,
    Icon,
    Button,
    Input,
    Textarea,
    Number,
    Slider,
    Switch,
    Select,
    Progress,
    Divider,
    Spacer,
}
#[derive(Debug, Deserialize)]
pub struct Node {
    id: String,
    #[serde(rename = "type")]
    kind: Kind,
    #[serde(default, deserialize_with = "optional_map")]
    props: Map<String, Value>,
    #[serde(default, deserialize_with = "optional_map")]
    layout: Map<String, Value>,
    #[serde(default, deserialize_with = "optional_map")]
    style: Map<String, Value>,
    #[serde(default, deserialize_with = "optional_map")]
    accessibility: Map<String, Value>,
    #[serde(default)]
    events: BTreeMap<String, String>,
    #[serde(default)]
    children: Vec<Node>,
    #[serde(skip)]
    button_style: std::sync::OnceLock<Map<String, Value>>,
    #[serde(skip)]
    textarea_style: std::sync::OnceLock<Map<String, Value>>,
    // 本地呈现投影：未解析资源隐藏；有内联来源但无像素时保留独立替代内容。
    #[serde(skip)]
    image_unresolved: bool,
    #[serde(skip)]
    image_placeholder: bool,
    // 仅显式 JS 呈现帧经私有尺寸验证后写入；普通声明式输入不能声明像素已存在。
    #[serde(skip)]
    canvas_size: Option<[u32; 2]>,
}
// Loom 的四项可选属性在缺省时序列化为 null；只将其视为空对象，不放宽其他类型或集合。
fn optional_map<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Map<String, Value>, D::Error> {
    Ok(Option::<Map<String, Value>>::deserialize(deserializer)?.unwrap_or_default())
}
impl Node {
    pub(crate) fn canvas_size(&self) -> Option<[u32; 2]> {
        self.canvas_size
    }
    /// 只判断旧 imageSource 的内联来源类别，不解析、下载或宣称可解码。
    pub(crate) fn inline_image_source(&self) -> Option<&str> {
        if self.kind != Kind::Image
            || self
                .props
                .get("resourceId")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty())
        {
            return None;
        }
        self.props
            .get("src")
            .and_then(Value::as_str)
            .filter(|src| src.starts_with("data:image/"))
    }
    pub(crate) fn image_placeholder(&self) -> bool {
        self.image_placeholder
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn props(&self) -> &Map<String, Value> {
        &self.props
    }
    pub fn visible(&self) -> bool {
        !self.image_unresolved && self.props.get("visible") != Some(&Value::Bool(false))
    }
    pub fn visible_children(&self) -> impl DoubleEndedIterator<Item = &Node> {
        self.children
            .iter()
            .filter(|child| !matches!(self.kind, Kind::Icon | Kind::Image) && child.visible())
    }
    pub(crate) fn visible_node(&self, id: &str) -> Option<&Self> {
        if self.id() == id {
            Some(self)
        } else {
            self.visible_children().find_map(|n| n.visible_node(id))
        }
    }
    pub fn layout(&self) -> &Map<String, Value> {
        if self.kind == Kind::Switch {
            return crate::surface_node_defaults::switch_properties();
        }
        if self.kind == Kind::Icon {
            return crate::surface_node_defaults::icon_layout();
        }
        if self.kind == Kind::Progress {
            return crate::surface_node_defaults::progress_layout();
        }
        if self.kind == Kind::Divider {
            crate::surface_node_defaults::divider_layout()
        } else {
            &self.layout
        }
    }
    /// 未提供或不在长度白名单中的值交回布局默认规则，不作为原始表达式执行。
    pub fn layout_length(&self, key: &str) -> Option<crate::surface_length::Length> {
        crate::surface_length::Length::parse(self.layout().get(key)?)
    }
    pub fn style_length(&self, key: &str) -> Option<crate::surface_length::Length> {
        crate::surface_length::Length::parse(self.style().get(key)?)
    }
    pub fn style(&self) -> &Map<String, Value> {
        if self.kind == Kind::Textarea {
            return self
                .textarea_style
                .get_or_init(|| crate::surface_textarea::style(&self.style));
        }
        if self.kind == Kind::Switch {
            return crate::surface_node_defaults::switch_properties();
        }
        if self.kind == Kind::Icon {
            return crate::surface_node_defaults::icon_style();
        }
        if self.kind == Kind::Button {
            return self.button_style.get_or_init(|| {
                crate::surface_button::style(
                    &self.style,
                    self.props.get("disabled") == Some(&Value::Bool(true)),
                )
            });
        }
        if self.kind == Kind::Progress {
            return crate::surface_node_defaults::progress_style();
        }
        if self.kind == Kind::Divider {
            crate::surface_node_defaults::divider_style()
        } else {
            &self.style
        }
    }
    pub fn accessibility(&self) -> &Map<String, Value> {
        &self.accessibility
    }
    pub(crate) fn explicit_style(&self, key: &str) -> bool {
        self.style.contains_key(key)
    }
    pub fn events(&self) -> &BTreeMap<String, String> {
        &self.events
    }
    pub fn children(&self) -> &[Node] {
        &self.children
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Identity,
    Duplicate,
    Runtime,
}
#[derive(Debug)]
pub struct Scene {
    root: Node,
    count: usize,
}
impl Scene {
    pub(crate) fn set_canvas_sizes(&mut self, sizes: &BTreeMap<String, [u32; 2]>) {
        fn visit(node: &mut Node, sizes: &BTreeMap<String, [u32; 2]>) {
            node.canvas_size = sizes.get(node.id()).copied();
            for child in &mut node.children {
                visit(child, sizes);
            }
        }
        visit(&mut self.root, sizes);
    }
    /// 仅投影图片可用性，不改原始属性/资源标识或授权；普通 parse 保持严格测量语义。
    pub fn with_image_availability(mut self, mut available: impl FnMut(&str) -> bool) -> Self {
        fn visit(node: &mut Node, available: &mut impl FnMut(&str) -> bool) {
            if node.kind == Kind::Image {
                let ready = available(&node.id);
                node.image_placeholder = !ready && node.inline_image_source().is_some();
                node.image_unresolved = !ready && !node.image_placeholder;
            }
            for child in &mut node.children {
                visit(child, available);
            }
        }
        // parse 已限制深度与节点总数，不保留第二棵树或复制 JSON/像素。
        visit(&mut self.root, &mut available);
        self
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let root: Node = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        let mut ids = BTreeSet::new();
        let mut pending = vec![(&root, 1)];
        while let Some((node, depth)) = pending.pop() {
            if depth > MAX_DEPTH || ids.len() >= MAX_NODES {
                return Err(Error::Limit);
            }
            if !identifier(&node.id) {
                return Err(Error::Identity);
            }
            if !ids.insert(node.id.as_str()) {
                return Err(Error::Duplicate);
            }
            if node.events.len() > 32
                || [&node.props, &node.layout, &node.style, &node.accessibility]
                    .iter()
                    .any(|map| map.len() > 128)
            {
                return Err(Error::Limit);
            }
            if node
                .events
                .iter()
                .any(|(event, action)| !identifier(event) || !identifier(action))
            {
                return Err(Error::Identity);
            }
            // 保留未知属性供各节点解析器处理；有界数据不意味着 CSS、资源路径或动作可信。
            pending.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
        let count = ids.len();
        Ok(Self { root, count })
    }
    pub fn root(&self) -> &Node {
        &self.root
    }
    pub fn node_count(&self) -> usize {
        self.count
    }
}
fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
#[cfg(test)]
#[path = "surface_scene_tests.rs"]
mod tests;
