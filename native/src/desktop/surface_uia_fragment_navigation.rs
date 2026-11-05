//! Fragment 关系与属性映射；挂载 EDIT 通过 HWND override 合并系统 provider。
use super::*;
use crate::surface_accessibility::{Direction, Entry, Handle};
use crate::surface_scene::Kind;

pub(super) fn native_edit(node: &Handle) -> bool {
    node.read().is_some_and(|entry| {
        matches!(
            entry.kind,
            Kind::Input | Kind::Textarea | Kind::Number | Kind::Select | Kind::Slider
        )
    })
}
fn skip_unmounted(
    object: &Provider,
    mut node: Option<Handle>,
    direction: Direction,
) -> Result<Option<Handle>> {
    while let Some(current) = node {
        if !native_edit(&current)
            || super::super::super::accessibility_bridge::native_edit::hwnd(object.hwnd, &current)?
                .is_some()
        {
            return Ok(Some(current));
        }
        node = current.relative(direction);
    }
    Ok(None)
}
pub(super) unsafe fn root(object: &Provider) -> IRawElementProviderSimple {
    if let Some(root) = &object.root {
        return root.clone();
    }
    let raw = (object as *const Provider).cast_mut().cast();
    add_ref(raw);
    IRawElementProviderSimple::from_raw(raw)
}
pub(super) unsafe fn node(object: &Provider, handle: Handle) -> IRawElementProviderSimple {
    let provider = Provider {
        vtable: &VTABLE,
        fragment: &fragment::VTABLE,
        fragment_root: &fragment::ROOT_VTABLE,
        hwnd_override: &hwnd_override::VTABLE,
        invoke: &invoke::VTABLE,
        invokable: handle
            .read()
            .is_some_and(|entry| entry.kind == Kind::Button),
        toggle: &toggle::VTABLE,
        toggleable: handle
            .read()
            .is_some_and(|entry| entry.kind == Kind::Switch),
        range: &range::VTABLE,
        rangeable: handle
            .read()
            .is_some_and(|entry| entry.kind == Kind::Slider),
        references: AtomicU32::new(1),
        alive: object.alive.clone(),
        hwnd: object.hwnd,
        thread: object.thread,
        scene: object.scene.clone(),
        node: Some(handle),
        root: Some(root(object)),
    };
    IRawElementProviderSimple::from_raw(Box::into_raw(Box::new(provider)).cast())
}
pub(super) unsafe fn navigate(
    object: &Provider,
    direction: NavigateDirection,
) -> Result<Option<IRawElementProviderFragment>> {
    let direction = match direction.0 {
        0 => Direction::Parent,
        1 => Direction::NextSibling,
        2 => Direction::PreviousSibling,
        3 => Direction::FirstChild,
        4 => Direction::LastChild,
        _ => return Err(Error::from_hresult(E_INVALIDARG)),
    };
    let handle = if let Some(handle) = &object.node {
        if matches!(direction, Direction::Parent) && handle.relative(direction).is_none() {
            return root(object).cast().map(Some);
        }
        let step = if matches!(direction, Direction::LastChild | Direction::PreviousSibling) {
            Direction::PreviousSibling
        } else {
            Direction::NextSibling
        };
        skip_unmounted(object, handle.relative(direction), step)?
    } else if matches!(direction, Direction::FirstChild | Direction::LastChild) {
        let handle = object
            .scene
            .lock()
            .map_err(|_| Error::from_hresult(E_FAIL))?
            .clone();
        skip_unmounted(
            object,
            handle.filter(|handle| handle.runtime_id().is_some()),
            Direction::NextSibling,
        )?
    } else {
        None
    };
    handle.map(|handle| node(object, handle).cast()).transpose()
}
pub(super) fn property(entry: &Entry, id: UIA_PROPERTY_ID) -> VARIANT {
    if id == UIA_NamePropertyId {
        VARIANT::from(entry.label.as_deref().unwrap_or(""))
    } else if id == UIA_AutomationIdPropertyId {
        VARIANT::from(entry.id.as_str())
    } else if id == UIA_FrameworkIdPropertyId {
        VARIANT::from("Otter.Native")
    } else if id == UIA_ControlTypePropertyId {
        let kind = match entry.kind {
            Kind::Text => UIA_TextControlTypeId,
            Kind::Button => UIA_ButtonControlTypeId,
            Kind::Switch => UIA_CheckBoxControlTypeId,
            Kind::Slider => UIA_SliderControlTypeId,
            Kind::Icon | Kind::Image => UIA_ImageControlTypeId,
            Kind::Progress => UIA_ProgressBarControlTypeId,
            _ => UIA_GroupControlTypeId,
        };
        VARIANT::from(kind.0)
    } else if id == UIA_IsControlElementPropertyId || id == UIA_IsContentElementPropertyId {
        // 无 alt 的非交互 Image 是纯装饰，保留 raw provider 但不污染 control/content view。
        VARIANT::from(
            entry.kind != Kind::Image
                || entry.label.as_ref().is_some_and(|label| !label.is_empty()),
        )
    } else {
        VARIANT::default()
    }
}
pub(super) unsafe fn point(
    object: &Provider,
    point: [f64; 2],
) -> Result<Option<IRawElementProviderFragment>> {
    use super::super::super::accessibility_bridge::{self, Hit};
    if point.iter().any(|value| !value.is_finite()) {
        return Err(Error::from_hresult(E_INVALIDARG));
    }
    let scene = object
        .scene
        .lock()
        .map_err(|_| Error::from_hresult(E_FAIL))?
        .clone();
    // 可见树为空时允许没有命中；不要把合法的空树变成损坏的 UIA 元素。
    let Some(scene) = scene else {
        return Ok(None);
    };
    match accessibility_bridge::hit(object.hwnd, &scene, point)? {
        Hit::Outside => Ok(None),
        Hit::Root => root(object).cast().map(Some),
        Hit::Node(handle) => node(object, handle).cast().map(Some),
    }
}
fn scene(object: &Provider) -> Result<Option<Handle>> {
    object
        .scene
        .lock()
        .map(|scene| scene.clone())
        .map_err(|_| Error::from_hresult(E_FAIL))
}
pub(super) unsafe fn focused(object: &Provider) -> Result<Option<IRawElementProviderFragment>> {
    use super::super::super::accessibility_bridge::focus::{self, Focus};
    match focus::get(object.hwnd, scene(object)?.as_ref())? {
        Focus::None => Ok(None),
        Focus::Root => root(object).cast().map(Some),
        Focus::Node(handle) => node(object, handle).cast().map(Some),
    }
}
pub(super) fn set_focus(object: &Provider) -> Result<()> {
    if let Some(node) = object.node.as_ref().filter(|node| native_edit(node)) {
        return super::super::super::accessibility_bridge::native_edit::focus(object.hwnd, node);
    }
    super::super::super::accessibility_bridge::focus::set(
        object.hwnd,
        scene(object)?.as_ref(),
        object.node.as_ref(),
    )
}
pub(super) fn focus_property(object: &Provider, id: UIA_PROPERTY_ID) -> Result<Option<VARIANT>> {
    if ![
        UIA_IsEnabledPropertyId,
        UIA_IsKeyboardFocusablePropertyId,
        UIA_HasKeyboardFocusPropertyId,
    ]
    .contains(&id)
    {
        return Ok(None);
    }
    let properties = super::super::super::accessibility_bridge::focus::properties(
        object.hwnd,
        scene(object)?.as_ref(),
        object.node.as_ref(),
    )?;
    Ok(Some(VARIANT::from(if id == UIA_IsEnabledPropertyId {
        properties.enabled
    } else if id == UIA_IsKeyboardFocusablePropertyId {
        properties.focusable
    } else {
        properties.focused
    })))
}
