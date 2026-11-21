//! 一页未连线文本原子提交；只推进本次同步提交的后缀，外部/模态变化不得获新凭证。
use super::*;

pub(super) fn enqueue(queue: &mut VecDeque<Intent>, intent: Intent) {
    let intent = match intent {
        Intent::Choice(snapshot, index, Some(text)) => Intent::Texts(snapshot, vec![(index, text)]),
        other => other,
    };
    if let Intent::Texts(snapshot, edits) = &intent {
        if let Some(Intent::Texts(previous, pending)) = queue.back_mut() {
            if Rc::ptr_eq(previous, snapshot) {
                for (index, text) in edits {
                    if let Some((_, old)) = pending.iter_mut().find(|(i, _)| i == index) {
                        *old = text.clone();
                    } else {
                        pending.push((*index, text.clone()));
                    }
                }
                return;
            }
        }
    }
    queue.push_back(intent);
}

pub(super) unsafe fn drain(panel: &Panel, state: &State) {
    let mut intents = panel.ui.pending.take();
    if !panel.ui.available() {
        return;
    }
    while let Some(intent) = intents.pop_front() {
        if !panel.ui.available() {
            break;
        }
        let result = if let Intent::Texts(snapshot, edits) = intent {
            submit(panel, state, &snapshot, &edits, &mut intents)
        } else {
            panel.apply(state, intent)
        };
        if let Err(error) = result {
            panel.ui.error(&format!("未完成：{error}"));
            break;
        }
    }
}

unsafe fn submit(
    panel: &Panel,
    state: &State,
    snapshot: &Rc<Snapshot>,
    edits: &[(usize, String)],
    remaining: &mut VecDeque<Intent>,
) -> Result<()> {
    let before = Stamp::capture(state);
    if !snapshot.submit_texts(panel.ui.host, state, edits)? {
        return Err(Error::new(E_ABORT, "配置已变化，草稿未提交"));
    }
    // submit_texts 没有模态/同步 Win32 回调；取消通知只是 PostMessage。
    // 全部值与原 Spec 验证完才发布，后继必须按稳定参数/动作重绑定，不能沿用旧索引。
    let after = Stamp::capture(state);
    let successor = Snapshot::capture(state).map(Rc::new);
    for binding in panel.ui.bindings.borrow_mut().iter_mut() {
        if let controls::Kind::Text {
            snapshot: bound,
            index,
            initial,
            ..
        } = &mut binding.kind
        {
            if Rc::ptr_eq(bound, snapshot) {
                if let Some((_, text)) = edits.iter().find(|(i, _)| i == index) {
                    *initial = text.clone();
                }
            }
        }
    }
    if let Some(successor) = &successor {
        advance_inline_drafts(panel, state, snapshot, successor);
    }
    for intent in remaining {
        match intent {
            Intent::Choice(bound, index, _) if Rc::ptr_eq(bound, snapshot) => {
                let successor = successor.as_ref().ok_or_else(stale)?;
                *index = successor
                    .corresponding(snapshot, *index)
                    .ok_or_else(stale)?;
                *bound = successor.clone();
            }
            Intent::Texts(bound, edits) if Rc::ptr_eq(bound, snapshot) => {
                let successor = successor.as_ref().ok_or_else(stale)?;
                for (index, _) in edits {
                    *index = successor
                        .corresponding(snapshot, *index)
                        .ok_or_else(stale)?;
                }
                *bound = successor.clone();
            }
            Intent::Setting(stamp, _)
            | Intent::Run(stamp)
            | Intent::Stop(stamp)
            | Intent::Ports(stamp)
            | Intent::Sticker(stamp, _)
                if *stamp == before =>
            {
                *stamp = after.clone();
            }
            _ => {}
        }
    }
    panel.ui.error("");
    Ok(())
}
fn stale() -> Error {
    Error::new(E_ABORT, "参数操作已变化，请重试")
}

unsafe fn advance_inline_drafts(
    panel: &Panel,
    state: &State,
    previous: &Rc<Snapshot>,
    successor: &Rc<Snapshot>,
) {
    if !panel.ui.inline || panel.ui.drafts_idle() {
        return;
    }
    // 只延续刚刚确认的本 owner 同步文本事务。外部/模态换代不能走到这里。
    // 后来的 thumb-track 草稿保留 HWND；结构变化仍沿原失效/重新加载路径。
    let mut next = panel.ui.capture(state);
    let old = panel.ui.model.borrow();
    if old.rows.len() != next.rows.len()
        || old.rows.iter().zip(&next.rows).any(|(a, b)| {
            a.id != b.id || a.widget != b.widget || a.primary.len() != b.primary.len()
        })
    {
        return;
    }
    drop(old);
    let mut bindings = panel.ui.bindings.borrow().clone();
    for binding in &mut bindings {
        let (bound, indices) = match &mut binding.kind {
            controls::Kind::Text {
                snapshot, index, ..
            }
            | controls::Kind::Slider(_, _, snapshot, index)
            | controls::Kind::Choice(snapshot, index) => (snapshot, std::slice::from_mut(index)),
            controls::Kind::Options(snapshot, indices) => (snapshot, indices.as_mut_slice()),
            _ => continue,
        };
        if !Rc::ptr_eq(bound, previous) {
            continue;
        }
        for index in indices {
            let Some(current) = successor.corresponding(previous, *index) else {
                return;
            };
            *index = current;
        }
        *bound = successor.clone();
    }
    next.snapshot = Some(successor.clone());
    *panel.ui.bindings.borrow_mut() = bindings;
    *panel.ui.model.borrow_mut() = Rc::new(next);
}
