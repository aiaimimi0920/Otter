//! Source恢复联合的自有socket断流/系统菜单/捕获身份检查，不添加产品故障命令。
use super::*;
#[derive(Default)]
pub(super) struct Progress {
    requested: bool,
    observed: bool,
    resumed: bool,
}
impl Progress {
    pub unsafe fn poll(
        &mut self,
        root: &std::path::Path,
        app: &mut Application,
        identity: &std::sync::Arc<()>,
        hwnd: HWND,
    ) {
        let mode = std::env::var("OTTER_LIVE_SOURCE_RECOVERY_JOINT").unwrap_or_default();
        if mode != "1" && mode != "control" {
            return;
        }
        if !self.requested && root.join("source-disconnect").exists() {
            if mode == "control" {
                // 只中断已建立的自有长轮询；请求间隙等下轮，不取消TLS握手或用户应用。
                self.requested = app.remote_sessions.test_disconnect_source_control(identity);
            } else {
                assert!(app.remote_sessions.test_disconnect_source(identity));
                self.requested = true;
            }
        }
        let recovering = app.remote_sessions.source_recovering(identity);
        let phase = if !self.observed && recovering {
            "source-disconnected.json"
        } else if self.observed
            && !self.resumed
            && !recovering
            && app.remote_sessions.source_status(identity) == Some((true, false))
        {
            "source-resumed.json"
        } else {
            return;
        };
        assert!(app.live_sessions[0].is_alive());
        assert!(std::sync::Arc::ptr_eq(
            &app.live_sessions[0].input_identity(),
            identity
        ));
        let mut text = [0u16; 128];
        let count = GetMenuStringW(
            GetSystemMenu(hwnd, false),
            PUBLISH as u32,
            Some(&mut text),
            MF_BYCOMMAND,
        );
        // 网络通知可在本次Application轮询之后到达；下轮核真实菜单，不用旧caption伪报错误。
        if String::from_utf16(&text[..count as usize]).unwrap()
            != if recovering {
                "Live 重连中…"
            } else {
                "Live 已发布"
            }
        {
            return;
        }
        std::fs::write(root.join(phase), serde_json::to_vec(&serde_json::json!({"recovering":recovering,"sameCapture":true,"sameSourceIdentity":true,"menuState":true})).unwrap()).unwrap();
        if recovering {
            self.observed = true;
        } else {
            self.resumed = true;
        }
    }
}
