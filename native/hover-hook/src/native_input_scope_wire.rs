//! 来源快照的固定标量槽；COUNT 显式区分无来源和缺字段，外层 COMMIT 最后发布。
use super::*;
use windows::core::{w, PCWSTR};

pub const COUNT: PCWSTR = w!("Otter.Live.Native.Scope.Count.v1");
const KEYS: [[PCWSTR; 11]; LIMIT] = [
    [
        w!("Otter.Live.Native.Scope.0.Root.v1"),
        w!("Otter.Live.Native.Scope.0.RootThread.v1"),
        w!("Otter.Live.Native.Scope.0.Target.v1"),
        w!("Otter.Live.Native.Scope.0.TargetThread.v1"),
        w!("Otter.Live.Native.Scope.0.Owner.v1"),
        w!("Otter.Live.Native.Scope.0.Cookie.v1"),
        w!("Otter.Live.Native.Scope.0.Epoch.v1"),
        w!("Otter.Live.Native.Scope.0.Anchor.v1"),
        w!("Otter.Live.Native.Scope.0.AnchorThread.v1"),
        w!("Otter.Live.Native.Scope.0.Request.v1"),
        w!("Otter.Live.Native.Scope.0.Instance.v1"),
    ],
    [
        w!("Otter.Live.Native.Scope.1.Root.v1"),
        w!("Otter.Live.Native.Scope.1.RootThread.v1"),
        w!("Otter.Live.Native.Scope.1.Target.v1"),
        w!("Otter.Live.Native.Scope.1.TargetThread.v1"),
        w!("Otter.Live.Native.Scope.1.Owner.v1"),
        w!("Otter.Live.Native.Scope.1.Cookie.v1"),
        w!("Otter.Live.Native.Scope.1.Epoch.v1"),
        w!("Otter.Live.Native.Scope.1.Anchor.v1"),
        w!("Otter.Live.Native.Scope.1.AnchorThread.v1"),
        w!("Otter.Live.Native.Scope.1.Request.v1"),
        w!("Otter.Live.Native.Scope.1.Instance.v1"),
    ],
    [
        w!("Otter.Live.Native.Scope.2.Root.v1"),
        w!("Otter.Live.Native.Scope.2.RootThread.v1"),
        w!("Otter.Live.Native.Scope.2.Target.v1"),
        w!("Otter.Live.Native.Scope.2.TargetThread.v1"),
        w!("Otter.Live.Native.Scope.2.Owner.v1"),
        w!("Otter.Live.Native.Scope.2.Cookie.v1"),
        w!("Otter.Live.Native.Scope.2.Epoch.v1"),
        w!("Otter.Live.Native.Scope.2.Anchor.v1"),
        w!("Otter.Live.Native.Scope.2.AnchorThread.v1"),
        w!("Otter.Live.Native.Scope.2.Request.v1"),
        w!("Otter.Live.Native.Scope.2.Instance.v1"),
    ],
    [
        w!("Otter.Live.Native.Scope.3.Root.v1"),
        w!("Otter.Live.Native.Scope.3.RootThread.v1"),
        w!("Otter.Live.Native.Scope.3.Target.v1"),
        w!("Otter.Live.Native.Scope.3.TargetThread.v1"),
        w!("Otter.Live.Native.Scope.3.Owner.v1"),
        w!("Otter.Live.Native.Scope.3.Cookie.v1"),
        w!("Otter.Live.Native.Scope.3.Epoch.v1"),
        w!("Otter.Live.Native.Scope.3.Anchor.v1"),
        w!("Otter.Live.Native.Scope.3.AnchorThread.v1"),
        w!("Otter.Live.Native.Scope.3.Request.v1"),
        w!("Otter.Live.Native.Scope.3.Instance.v1"),
    ],
];

pub fn keys() -> impl Iterator<Item = PCWSTR> {
    std::iter::once(COUNT).chain(KEYS.into_iter().flatten())
}
impl Link {
    fn values(self) -> [usize; 11] {
        [
            self.root,
            self.root_thread,
            self.target,
            self.target_thread,
            self.owner,
            self.cookie,
            self.epoch,
            self.anchor,
            self.anchor_thread,
            self.request,
            self.instance,
        ]
    }
    fn from_values(v: [usize; 11]) -> Self {
        Self {
            root: v[0],
            root_thread: v[1],
            target: v[2],
            target_thread: v[3],
            owner: v[4],
            cookie: v[5],
            epoch: v[6],
            anchor: v[7],
            anchor_thread: v[8],
            request: v[9],
            instance: v[10],
        }
    }
}
impl Snapshot {
    pub fn keys(self) -> impl Iterator<Item = PCWSTR> {
        std::iter::once(COUNT).chain(KEYS.into_iter().take(self.len).flatten())
    }
    pub fn read(owner: HWND) -> Option<Self> {
        let len = native_protocol::value(owner, COUNT).checked_sub(1)?;
        if len > LIMIT {
            return None;
        }
        let mut snapshot = Self::default();
        for keys in KEYS.iter().take(len) {
            snapshot.push(Link::from_values(
                keys.map(|key| native_protocol::value(owner, key)),
            ));
        }
        Some(snapshot)
    }
    /// 调用方拥有该 B 的单在途槽，并已撤 COMMIT；这里只写原值，不读当前来源重新授权。
    pub fn publish(self, owner: HWND) -> windows::core::Result<()> {
        unsafe {
            let _ = RemovePropW(owner, COUNT);
            for (keys, link) in KEYS.iter().zip(self.links.iter()).take(self.len) {
                for (key, value) in keys.iter().zip(link.values()) {
                    if value != 0 {
                        SetPropW(owner, *key, Some(HANDLE(value as *mut _)))?;
                    } else {
                        let _ = RemovePropW(owner, *key);
                    }
                }
            }
            SetPropW(owner, COUNT, Some(HANDLE((self.len + 1) as *mut _)))
        }
    }
}
