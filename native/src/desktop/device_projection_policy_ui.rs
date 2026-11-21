//! 所有窗口共用设备同意和不可复活的请求凭证；菜单只排队，不读写文件。
use super::*;
use crate::projection_receive_policy::Policy;
use std::rc::Rc;

pub(in crate::desktop) const AUTO: usize = 0x1320;
pub(in crate::desktop) const CONFIRM: usize = 0x1321;
pub(in crate::desktop) const REJECT: usize = 0x1322;
pub(in crate::desktop) const PAUSE: usize = 0x1323;

#[derive(Clone)]
pub(in crate::desktop) struct Stamp(Rc<()>);
#[derive(Clone)]
pub(in crate::desktop) struct Participation(Rc<()>);
#[derive(Default)]
pub(in crate::desktop) struct Consent {
    policy: Policy,
    generation: Rc<()>,
    participation: Rc<()>,
    paused: bool,
    pausing: bool,
    pub saving: bool,
    pub status: &'static str,
}
impl Consent {
    pub fn policy(&self) -> Policy {
        self.policy
    }
    pub fn stamp(&self) -> Option<Stamp> {
        (!self.saving && !self.paused && self.policy != Policy::Reject)
            .then(|| Stamp(self.generation.clone()))
    }
    pub fn matches(&self, stamp: &Stamp) -> bool {
        !self.saving
            && !self.paused
            && self.policy != Policy::Reject
            && Rc::ptr_eq(&self.generation, &stamp.0)
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn participation(&self) -> Option<Participation> {
        (!self.paused && !self.pausing).then(|| Participation(self.participation.clone()))
    }
    pub fn participates(&self, stamp: &Participation) -> bool {
        !self.paused && !self.pausing && Rc::ptr_eq(&self.participation, &stamp.0)
    }
    pub fn install_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.participation = Rc::new(());
            self.generation = Rc::new(());
        }
        self.paused = paused;
    }
    pub fn begin_pause(&mut self) {
        self.begin_save();
        self.participation = Rc::new(());
        self.pausing = true;
    }
    pub fn finish_save(&mut self) {
        self.saving = false;
        self.pausing = false;
    }
    pub fn install(&mut self, policy: Policy) {
        if self.policy != policy {
            self.generation = Rc::new(());
        }
        self.policy = policy;
    }
    pub fn begin_save(&mut self) {
        // 即使写失败，已在途的新接受也不复活；内存策略值仍到 commit 成功才更新。
        self.generation = Rc::new(());
        self.saving = true;
        self.status = "正在保存接收策略…";
    }
}
pub(super) fn handles(command: usize) -> bool {
    matches!(command, AUTO | CONFIRM | REJECT | PAUSE)
}
pub(super) fn request(view: &View, command: usize) -> Result<Request> {
    if view.closed || view.request.is_some() || view.consent.saving {
        return Err(Error::new(E_ABORT, "Receive policy unavailable"));
    }
    if command == PAUSE {
        return Ok(Request::ReceivePause(true));
    }
    let policy = match command {
        AUTO => Policy::Auto,
        CONFIRM => Policy::Confirm,
        REJECT => Policy::Reject,
        _ => return Err(Error::new(E_INVALIDARG, "Unknown receive policy")),
    };
    Ok(Request::ReceivePolicy(policy))
}
pub(super) unsafe fn append_menu(menu: HMENU, view: &View) -> Result<()> {
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    for (id, selected, label) in [
        (PAUSE, view.consent.paused(), "暂停"),
        (
            CONFIRM,
            !view.consent.paused() && view.consent.policy() == Policy::Confirm,
            "接收",
        ),
        (
            REJECT,
            !view.consent.paused() && view.consent.policy() == Policy::Reject,
            "拒绝",
        ),
        (
            AUTO,
            !view.consent.paused() && view.consent.policy() == Policy::Auto,
            "自动",
        ),
    ] {
        let enabled = !view.closed && !view.consent.saving && view.request.is_none();
        AppendMenuW(
            menu,
            MF_STRING
                | if enabled { MF_ENABLED } else { MF_GRAYED }
                | if selected { MF_CHECKED } else { MF_UNCHECKED },
            id,
            &HSTRING::from(label),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receive_policy_stamp_never_revives_after_away_back_or_failed_save() {
        let mut consent = Consent::default();
        let old = consent.stamp().unwrap();
        consent.install(Policy::Auto);
        consent.install(Policy::Confirm);
        assert!(!consent.matches(&old));
        let current = consent.stamp().unwrap();
        consent.begin_save();
        assert_eq!(consent.policy(), Policy::Confirm);
        assert!(consent.stamp().is_none());
        consent.saving = false;
        assert!(!consent.matches(&current));
        consent.install(Policy::Reject);
        assert!(consent.stamp().is_none());
    }
    #[test]
    fn receive_policy_participation_survives_new_consent_but_never_pause_away_back() {
        let mut consent = Consent::default();
        let participation = consent.participation().unwrap();
        consent.begin_save();
        consent.install(Policy::Reject);
        assert!(consent.participates(&participation));
        consent.finish_save();
        consent.begin_pause();
        assert!(!consent.participates(&participation));
        assert!(consent.participation().is_none());
        consent.finish_save(); // 写失败也不能复活旧关联重建。
        assert!(!consent.participates(&participation));
        let next = consent.participation().unwrap();
        consent.install_paused(true);
        assert!(consent.stamp().is_none());
        consent.install_paused(false);
        assert!(!consent.participates(&next));
        assert_eq!(consent.policy(), Policy::Reject);
    }
}
