//! 同一文档响应与本地视图的纯合并；不写盘/发网/确认checkpoint，不静默重基同对象冲突。
use super::*;
use crate::projection_edit::Code;

#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    code: Code,
    changes: Vec<Change>,
}
impl Conflict {
    pub fn code(&self) -> Code {
        self.code
    }
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Reconciliation {
    authoritative: View,
    visible: View,
    changes: Vec<Change>,
    conflict: Option<Conflict>,
}
impl Reconciliation {
    /// 上层必须持久化这一权威基线，再在应用前重新观察本地view；本结果不签发落盘回执。
    pub fn authoritative(&self) -> &View {
        &self.authoritative
    }
    pub fn visible(&self) -> &View {
        &self.visible
    }
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }
    pub fn conflict(&self) -> Option<&Conflict> {
        self.conflict.as_ref()
    }
}

/// observed是已确认基线（apply响应对应其原pending view）；current须在每次异步保存后重读。
/// 重调此纯函数会包含期间的新编辑；取消/宿主身份和真正checkpoint仍由Application拥有。
pub fn reconcile(
    previous: &Document,
    observed: &View,
    document: &Document,
    current: &View,
    sticky: Option<Code>,
) -> Result<Reconciliation, Error> {
    previous.validate().map_err(Error::Protocol)?;
    document.validate().map_err(Error::Protocol)?;
    document.follows(previous).map_err(Error::Protocol)?;
    if observed.basis() != previous.basis() || current.basis() != previous.basis() {
        return Err(Error::Protocol(super::super::Error::Basis));
    }
    let changes = observed.changes_to(current)?;
    let remote = View::from_document(document, current.size())?;
    let race = changes.iter().any(|change| {
        !equal_optional(
            observed.basis_objects().get(&change.object_id),
            remote.basis_objects().get(&change.object_id),
        )
    });
    // current是唯一完整本地真值；不把旧conflict payload重新叠回，否则undo会复活旧编辑。
    validate_changes(&changes)?;
    let visible = remote.overlay(&changes)?;
    let code = sticky.or(race.then_some(Code::ObjectConflict));
    let conflict = code.map(|code| Conflict {
        code,
        changes: changes.clone(),
    });
    Ok(Reconciliation {
        authoritative: remote,
        visible,
        changes,
        conflict,
    })
}
