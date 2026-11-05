//! 跨弹出窗口的逻辑树；双向均为弱关联，不保活已关闭面板或 COM provider。
use super::*;
#[derive(Clone)]
pub(super) struct WeakNode {
    snapshot: std::sync::Weak<Mutex<Snapshot>>,
    generation: u64,
    index: usize,
}
impl Owner {
    pub(crate) fn set_location(&self, hwnd: usize, row: i32) {
        if let Ok(mut state) = self.0.lock() {
            state.location = Some((hwnd, row));
        }
    }
    pub(crate) fn clear_links(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.parent = None;
            state.child = None;
        }
    }
    pub(crate) fn link_child(&self, child: &Owner, key: usize) {
        // 不同时持两把锁，避免跨线程读者形成反向锁序。
        let parent = {
            let Ok(mut state) = self.0.lock() else { return };
            if !state.alive {
                return;
            }
            let Some(index) = state.keys.iter().position(|value| *value == key) else {
                return;
            };
            state.child = Some((index, Arc::downgrade(&child.0)));
            WeakNode {
                snapshot: Arc::downgrade(&self.0),
                generation: state.generation,
                index,
            }
        };
        if let Ok(mut state) = child.0.lock() {
            if state.alive {
                state.parent = Some(parent);
            }
        }
    }
}
impl View {
    pub(crate) fn same(&self, other: &View) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn focused(&self) -> Option<(View, Node)> {
        let mut view = self.clone();
        let mut focused = None;
        // 与菜单深度预算一致，即使关联损坏也不会无限遍历。
        for _ in 0..16 {
            if let Some(node) = view
                .nodes()
                .into_iter()
                .find(|node| node.read().is_some_and(|entry| entry.focused))
            {
                focused = Some((view.clone(), node));
            }
            let next = {
                let state = view.0.lock().ok()?;
                if !state.alive {
                    break;
                }
                state.child.as_ref().and_then(|(_, child)| child.upgrade())
            };
            let Some(next) = next else { break };
            view = View(next);
        }
        focused
    }
    pub(crate) fn location(&self) -> Option<(usize, i32)> {
        let state = self.0.lock().ok()?;
        state.alive.then_some(state.location).flatten()
    }
    pub(crate) fn parent(&self) -> Option<(View, Node)> {
        let parent = {
            let state = self.0.lock().ok()?;
            if !state.alive {
                return None;
            }
            state.parent.clone()?
        };
        let snapshot = parent.snapshot.upgrade()?;
        let node = Node {
            snapshot: snapshot.clone(),
            generation: parent.generation,
            index: parent.index,
        };
        node.read()?;
        Some((View(snapshot), node))
    }
}
impl Node {
    pub(crate) fn child(&self) -> Option<View> {
        let weak = {
            let state = self.snapshot.lock().ok()?;
            if !state.alive || state.generation != self.generation {
                return None;
            }
            let (index, weak) = state.child.as_ref()?;
            if *index != self.index {
                return None;
            }
            weak.clone()
        };
        let view = View(weak.upgrade()?);
        view.alive().then_some(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weak_relationships_do_not_keep_closed_or_replaced_nodes_available() {
        let rows = vec![Row {
            key: 7,
            thumbnail: None,
            label: "parent".into(),
            enabled: true,
            checked: false,
            submenu: true,
            separator: false,
            focused: false,
            radio: false,
            default: false,
        }];
        let parent = Owner::new(&rows);
        let child = Owner::new(&[]);
        parent.link_child(&child, 7);
        let node = parent.nodes().remove(0);
        assert!(node.child().is_some());
        let view = child.view();
        assert_eq!(view.parent().unwrap().1.read().unwrap().name, "parent");
        parent.replace(&[]);
        assert!(view.parent().is_none());
        assert!(node.child().is_none());
        drop(parent);
        drop(child);
        assert!(!view.alive());
        assert!(view.parent().is_none());
    }
}
