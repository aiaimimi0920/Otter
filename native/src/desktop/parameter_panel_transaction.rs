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
    if before != after {
        panel.ui.committed.set(true);
    }
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
            | Intent::Relay(stamp)
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
