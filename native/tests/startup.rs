//! 子进程验证真实 Win32/D2D 启动与关闭；不操作用户桌面或正式配置。
#![cfg(windows)]
use std::process::Command;
#[path = "support/process.rs"]
mod process;

fn run_bounded(command: &mut Command) -> std::process::Output {
    // 三个完整 smoke 共用桌面/图形资源；libtest 并发会把独立启动功能回归变成负载测试。
    // 只串行本测试二进制的子进程，不减少场景/断言或放宽每个 child 的 15 秒期限。
    // 失败不能毒化其余场景；等待准入不计入尚未启动 child 的运行期限。
    static DESKTOP_SCENARIO: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _scenario = DESKTOP_SCENARIO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    process::run(command, std::time::Duration::from_secs(15)).unwrap()
}

fn assert_default_shader_is_configured_without_execution(stdout: &str) {
    // 原生路由默认可用不等于启动worker；无显式peer时不能取码或提交正式Shader结果。
    let configured: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("native-shader-startup:"))
        .collect();
    assert_eq!(
        configured,
        ["native-shader-startup: native=true application=true default=true"]
    );
    assert!(!stdout.contains("native-shader-product-smoke:"));
}

#[test]
fn capture_rejects_conflicting_sources_before_reading_desktop() {
    for extra in ["--smoke-test", "unused-image.png"] {
        let output = run_bounded(
            Command::new(env!("CARGO_BIN_EXE_otter-native")).args(["--capture", extra]),
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--capture cannot be combined"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-smoke:"));
    }
}

#[test]
fn hidden_window_renders_and_destroys_without_webview() {
    let output = run_bounded(Command::new(env!("CARGO_BIN_EXE_otter-native")).arg("--smoke-test"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_default_shader_is_configured_without_execution(&stdout);
    assert!(stdout.contains("destroyed=true webview=false"));
    assert!(
        stdout.contains("native-retirement-smoke:")
            && stdout.contains("document-preserved=true paints-preserved=true")
    );
    assert!(stdout.contains("native-workspace-autosave-smoke: menu=true shared=true worker=true unchanged-noop=true modal=true draft=true restored=true workflow=false"));
    assert!(stdout.contains("native-recycle-settings-smoke: menu=true cancel=true invalid=true maintenance=true save=true autosave=true stale=true owner-closed=true dialog-injected=true"));
    assert!(stdout.contains("native-recycle-maintenance-smoke: load=true settings=true noop-clean=true cancel=true stale=true policy-stale=true save=true windows-preserved=true"));
    assert!(stdout.contains("native-recycle-close-smoke: menu=true cancel=true stale=true close-failure=true retired=true last-host=true save=true restore=true"));
    assert!(stdout.contains("native-recycle-prepare-smoke: image=true typed=true single-slot=true late-cancel=true stale=true unpublished=true windows-preserved=true"));
    assert!(stdout.contains("native-frozen-remove-smoke: menu=true paged=true both-libraries=true cancel=true stale=true windows-preserved=true disk-revision=3"));
    assert!(stdout.contains("native-reference-disk-smoke: create=true update=true copy=true remove=true revision=4 external-conflict=true"));
    assert!(stdout.contains("native-reference-write-smoke: menu=true current-snapshot=true worker=true replace=true remove=true stale=true cancel=true revision-preserved=true"));
    assert!(stdout.contains("native-frozen-menu-smoke: indexed=true paged=true selection=true queued=true worker=true appended=true libraries=true"));
    assert!(stdout.contains("native-frozen-worker-smoke: worker=true append=true cancel=true stale=true coordinates=true recycle=true reference=true cleanup=true"));
    assert!(stdout.contains("native-frozen-append-smoke: append=true old-windows=true topology=true stale-rejected=true recycle-commit=true save=true cleanup=true"));
    assert!(stdout.contains(
        "native-webp-child-smoke: isolated=true pixels=true profile=false network=false"
    ));
    assert!(stdout
        .contains("native-svg-child-smoke: isolated=true pixels=true profile=false network=false"));
    assert!(stdout.contains("native-workflow-prepare-smoke: worker=true single-slot=true cancel=true stale=true receipt-retained=true network=false"));
    assert!(stdout.contains("native-rebuild-smoke: paints=2"));
    assert!(stdout.contains("native-save-state-smoke: committed-unchanged=true presentation-tracked=true transient-ignored=true draft-tracked=true space-tracked=true"));
    assert!(stdout.contains("native-exit-save-smoke: cancel-preserved=true save-queued=true failure-preserved=true changed-preserved=true clean-resumed=true"));
    assert!(stdout.contains("native-exit-save-ui-smoke: committed=true deferred=true stale-preserved=true queued-preserved=true dialogs=false"));
    assert!(stdout.contains("native-open-session-ui-smoke: menu=true coalesced=true modal=true cancel=true stale=true failed-preserved=true restored=true dialogs=false"));
    assert!(stdout.contains("native-save-open-smoke: deferred=true stale-preserved=true committed=true restored=true dialogs=false"));
    assert!(stdout.contains("native-session-context-smoke: cancel-preserved=true stale-rejected=true explicit-installed=true dialogs=false"));
    assert!(stdout.contains(
        "native-naming-input-smoke: cancel=true invalid-preserved=true shared=true dialogs=false"
    ));
    assert!(stdout
        .contains("native-save-menu-smoke: locked-save=true coalesced=true save-as-priority=true"));
    assert!(stdout.contains("native-save-replace-smoke: worker=true baseline-updated=true late-stale-preserved=true handles-released=true"));
    assert!(stdout.contains("native-save-view-smoke: viewport-stale=true invalid-rejected=true dpi-captured=true context-stale=true"));
    assert!(stdout
        .contains("native-save-art-smoke: retained-secret-filtered=true runtime-parameters=true"));
    assert!(
        stdout.contains("native-save-images-smoke: worker=true data-url=true restored-pixels=true")
    );
    assert!(stdout.contains("native-save-snapshot-smoke: shared-source=true worker-send=true modal-rejected=true stale-rejected=true isolated=true"));
    assert!(stdout.contains("native-unsaved-mini-smoke: current-frame=true normal-opacity=true document-preserved=true hidden=true"));
    assert!(stdout.contains("native-restored-rebind-smoke: modal-deferred=true secret-filtered=true no-execution=true catalog-retry=true selection-preserved=true"));
    assert!(stdout
        .contains("native-multi-host-smoke: independent=true close-one=true remaining-paint=true"));
    assert!(stdout.contains("native-multi-host-exit-smoke: last-window=true"));
    assert!(stdout.contains("native-application-exit-smoke: all-windows=true modal-deferred=true coalesced=true collected=true"));
    assert!(stdout.contains("native-workspace-identity-smoke: unique=true edit-stable=true closed-unresolved=true reopen-distinct=true"));
    assert!(stdout.contains("native-workspace-registry-smoke: import-synced=true close-cascades-links=true exit-cleared=true"));
    assert!(stdout.contains("native-workspace-groups-smoke: close-clears-membership=true remaining-member-preserved=true exit-cleared=true"));
    assert!(stdout.contains("native-group-commands-smoke: create=true join=true rename=true leave=true delete-preserves=true close-members=true modal-deferred=true"));
    assert!(stdout.contains("native-group-lock-smoke: busy-rejected=true members-only=true pointer-blocked=true keyboard-blocked=true unlocked-edit=true system-input=false"));
    assert!(stdout.contains("native-group-visibility-smoke: members-only=true failure-rollback=true tray-restores-group=true state-restored=true system-tray=false"));
    assert!(stdout.contains("native-group-catalog-smoke: empty-rename=true empty-delete=true empty-close=true unrelated-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-execution-owner-smoke: close-retires=true stale-rejected=true unrelated-preserved=true exit-cleared=true network=false"));
    assert!(stdout.contains("native-default-capture-smoke: new-pin=true modal-disabled=true recovered=true desktop-access=false"));
    assert!(stdout.contains(
        "native-pin-origin-smoke: negative-origin=true physical-size=true desktop-access=false"
    ));
    assert!(stdout.contains("native-new-pin-smoke: worker=true new-window=true shared-pixels=true source-history=true desktop-access=false"));
    assert!(stdout.contains("native-paste-pin-smoke: png=true dib=true file=true source-preserved=true pending-bounded=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-batch-smoke: count=2 source-preserved=true replacement-rejected=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-editable-smoke: independent-units=true fresh-annotations=true crop=true opacity=true undo-isolated=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-capture-smoke: metadata=true unknown-fields=true new-document-binding=true path-access=false"));
    assert!(stdout.contains("native-paste-placement-smoke: physical-size=true cascade=true pending-refresh=true failed-staging=true new-copy-isolated=true clipboard-access=false"));
    assert!(stdout.contains("native-history-delivery-smoke: text=true single-undo=true stale-rejected=true modal-rejected=true independent-pin=true clipboard-access=false"));
    assert!(stdout.contains("native-open-pin-smoke: worker=true new-window=true source-preserved=true cancelled=true failed=true stale-rejected=true dialog-access=false"));
    assert!(stdout
        .contains("native-open-batch-smoke: count=2 source-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-drop-pin-smoke: shell=true worker=true new-window=true source-preserved=true close-independent=true system-input=false"));
    assert!(stdout.contains(
        "native-drop-batch-smoke: count=2 one-worker=true source-preserved=true system-input=false"
    ));
    assert!(stdout.contains("native-import-rollback-smoke: staged-host-released=true staged-panel-released=true collection-unchanged=true"));
    assert!(stdout.contains("native-shared-modal-smoke: capture-blocked=true local-independent=true released=true desktop-access=false"));
    assert!(stdout.contains("native-shared-capture-smoke: windows=4 unique=true hidden-preserved=true shared-gate=true desktop-access=false"));
    assert!(stdout.contains("native-application-hotkey-smoke: shared-menu=true coalesced=true cross-window=true failure-preserved=true system-registration=false"));
    assert!(stdout.contains("native-application-hotkey-secondary-smoke: live=true long=true unique-owner=true cleanup=true system-registration=false"));
    assert!(stdout.contains("native-restored-hotkey-transfer-smoke: live=true long=true rollback=true unique-owner=true system-registration=false"));
    assert!(stdout.contains("native-application-smoke: command=true coalesced=true close-initial=true reopen=true collected=true"));
}

#[test]
fn wic_image_upload_survives_two_paints() {
    let image =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/icons/32x32.png");
    let output = run_bounded(
        Command::new(env!("CARGO_BIN_EXE_otter-native"))
            .arg(image)
            .arg("--smoke-test"),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_default_shader_is_configured_without_execution(&stdout);
    assert!(stdout.contains("destroyed=true webview=false"));
    assert!(
        stdout.contains("native-retirement-smoke:")
            && stdout.contains("document-preserved=true paints-preserved=true")
    );
    assert!(stdout.contains(
        "native-copy-smoke: prepared=true once=true escape=true stale=true clipboard-access=false"
    ));
    assert!(stdout.contains("native-file-dialog-smoke: open-blocked=true save-blocked=true drop-blocked=true recovered=true"));
    assert!(stdout
        .contains("native-drop-smoke: shell=true worker=true timer=true source-preserved=true"));
    assert!(stdout.contains("native-capture-input-smoke: cancel=true failure=true stale=true timer=true desktop-access=false"));
    assert!(stdout.contains("native-rebuild-smoke: paints=2"));
    assert!(stdout.contains("native-tool-panel-smoke: controls=19 click=true command=true"));
    assert!(stdout.contains("native-effect-controls-smoke: high-bits=true tool-gates=true"));
    assert!(stdout.contains(
        "native-panel-scroll-smoke: horizontal=true vertical=true reveal=true restored=true"
    ));
    assert!(stdout.contains("native-icon-smoke: host=true panel=true embedded=true"));
    assert!(stdout
        .contains("native-multi-host-smoke: independent=true close-one=true remaining-paint=true"));
    assert!(stdout.contains("native-multi-host-exit-smoke: last-window=true"));
    assert!(stdout.contains("native-application-exit-smoke: all-windows=true modal-deferred=true coalesced=true collected=true"));
    assert!(stdout.contains("native-workspace-identity-smoke: unique=true edit-stable=true closed-unresolved=true reopen-distinct=true"));
    assert!(stdout.contains("native-workspace-registry-smoke: import-synced=true close-cascades-links=true exit-cleared=true"));
    assert!(stdout.contains("native-workspace-groups-smoke: close-clears-membership=true remaining-member-preserved=true exit-cleared=true"));
    assert!(stdout.contains("native-group-commands-smoke: create=true join=true rename=true leave=true delete-preserves=true close-members=true modal-deferred=true"));
    assert!(stdout.contains("native-group-lock-smoke: busy-rejected=true members-only=true pointer-blocked=true keyboard-blocked=true unlocked-edit=true system-input=false"));
    assert!(stdout.contains("native-group-visibility-smoke: members-only=true failure-rollback=true tray-restores-group=true state-restored=true system-tray=false"));
    assert!(stdout.contains("native-group-catalog-smoke: empty-rename=true empty-delete=true empty-close=true unrelated-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-execution-owner-smoke: close-retires=true stale-rejected=true unrelated-preserved=true exit-cleared=true network=false"));
    assert!(stdout.contains("native-default-capture-smoke: new-pin=true modal-disabled=true recovered=true desktop-access=false"));
    assert!(stdout.contains(
        "native-pin-origin-smoke: negative-origin=true physical-size=true desktop-access=false"
    ));
    assert!(stdout.contains("native-new-pin-smoke: worker=true new-window=true shared-pixels=true source-history=true desktop-access=false"));
    assert!(stdout.contains("native-paste-pin-smoke: png=true dib=true file=true source-preserved=true pending-bounded=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-batch-smoke: count=2 source-preserved=true replacement-rejected=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-editable-smoke: independent-units=true fresh-annotations=true crop=true opacity=true undo-isolated=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-capture-smoke: metadata=true unknown-fields=true new-document-binding=true path-access=false"));
    assert!(stdout.contains("native-paste-placement-smoke: physical-size=true cascade=true pending-refresh=true failed-staging=true new-copy-isolated=true clipboard-access=false"));
    assert!(stdout.contains("native-history-delivery-smoke: text=true single-undo=true stale-rejected=true modal-rejected=true independent-pin=true clipboard-access=false"));
    assert!(stdout.contains("native-open-pin-smoke: worker=true new-window=true source-preserved=true cancelled=true failed=true stale-rejected=true dialog-access=false"));
    assert!(stdout
        .contains("native-open-batch-smoke: count=2 source-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-drop-pin-smoke: shell=true worker=true new-window=true source-preserved=true close-independent=true system-input=false"));
    assert!(stdout.contains(
        "native-drop-batch-smoke: count=2 one-worker=true source-preserved=true system-input=false"
    ));
    assert!(stdout.contains("native-import-rollback-smoke: staged-host-released=true staged-panel-released=true collection-unchanged=true"));
    assert!(stdout.contains("native-shared-modal-smoke: capture-blocked=true local-independent=true released=true desktop-access=false"));
    assert!(stdout.contains("native-shared-capture-smoke: windows=4 unique=true hidden-preserved=true shared-gate=true desktop-access=false"));
    assert!(stdout.contains("native-application-hotkey-smoke: shared-menu=true coalesced=true cross-window=true failure-preserved=true system-registration=false"));
    assert!(stdout.contains("native-application-hotkey-secondary-smoke: live=true long=true unique-owner=true cleanup=true system-registration=false"));
    assert!(stdout.contains("native-restored-hotkey-transfer-smoke: live=true long=true rollback=true unique-owner=true system-registration=false"));
    assert!(stdout.contains("native-application-smoke: command=true coalesced=true close-initial=true reopen=true collected=true"));
    assert!(
        stdout.contains("native-document-owner-smoke: relocated=true state-stable=true icons=true")
    );
    assert!(stdout.contains(
        "native-capture-protection-smoke: host=true panel=true reset=true desktop-access=false"
    ));
    assert!(stdout.contains(
        "native-tool-panel-availability-smoke: disabled=true queued-rejected=true recovered=true"
    ));
    assert!(stdout.contains(
        "native-tool-properties-smoke: controls=16 width=true opacity=true fill=true labels=true"
    ));
    assert!(stdout.contains(
        "native-tool-panel-refresh-smoke: unchanged-timers=8 refreshed-after-change=true"
    ));
    assert!(stdout.contains("native-marquee-smoke: pointer=true shift=true capture=true escape=true wheel=true undo=true"));
    assert!(stdout.contains("native-cancel-smoke: escape=true ctrl-escape=true"));
    assert!(stdout.contains(
        "native-crop-effect-smoke: timer=true atomic-view=true reset=true escape=true window=true flip=true"
    ));
    assert!(stdout.contains("annotations=4 device-recreated=true"));
    assert!(stdout.contains("native-flip-smoke: horizontal=true vertical=true timer=true undo=true redo=true escape=true"));
}

#[test]
fn unknown_flags_fail_before_starting_a_window() {
    let output =
        run_bounded(Command::new(env!("CARGO_BIN_EXE_otter-native")).arg("--unknown-option"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Usage:"));
}

#[test]
fn transparent_pin_creates_compositor_and_rebuilds_device_resources() {
    let image =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/icons/32x32.png");
    let output = run_bounded(
        Command::new(env!("CARGO_BIN_EXE_otter-native"))
            .arg(image)
            .arg("--pin")
            .arg("--smoke-test"),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_default_shader_is_configured_without_execution(&stdout);
    assert!(stdout.contains("pinned=true"));
    assert!(
        stdout.contains("native-retirement-smoke:")
            && stdout.contains("document-preserved=true paints-preserved=true")
    );
    assert!(stdout.contains(
        "native-copy-smoke: prepared=true once=true escape=true stale=true clipboard-access=false"
    ));
    assert!(stdout.contains("native-file-dialog-smoke: open-blocked=true save-blocked=true drop-blocked=true recovered=true"));
    assert!(stdout
        .contains("native-drop-smoke: shell=true worker=true timer=true source-preserved=true"));
    assert!(stdout.contains("native-capture-input-smoke: cancel=true failure=true stale=true timer=true desktop-access=false"));
    assert!(stdout.contains("native-cancel-smoke: escape=true ctrl-escape=true"));
    assert!(stdout.contains("annotations=4 device-recreated=true"));
    assert!(stdout.contains("native-rebuild-smoke: paints=2"));
    assert!(stdout.contains("native-tool-panel-smoke: controls=19 click=true command=true"));
    assert!(stdout.contains("native-effect-controls-smoke: high-bits=true tool-gates=true"));
    assert!(stdout.contains(
        "native-panel-scroll-smoke: horizontal=true vertical=true reveal=true restored=true"
    ));
    assert!(stdout.contains("native-icon-smoke: host=true panel=true embedded=true"));
    assert!(stdout
        .contains("native-multi-host-smoke: independent=true close-one=true remaining-paint=true"));
    assert!(stdout.contains("native-multi-host-exit-smoke: last-window=true"));
    assert!(stdout.contains("native-application-exit-smoke: all-windows=true modal-deferred=true coalesced=true collected=true"));
    assert!(stdout.contains("native-workspace-identity-smoke: unique=true edit-stable=true closed-unresolved=true reopen-distinct=true"));
    assert!(stdout.contains("native-workspace-registry-smoke: import-synced=true close-cascades-links=true exit-cleared=true"));
    assert!(stdout.contains("native-workspace-groups-smoke: close-clears-membership=true remaining-member-preserved=true exit-cleared=true"));
    assert!(stdout.contains("native-group-commands-smoke: create=true join=true rename=true leave=true delete-preserves=true close-members=true modal-deferred=true"));
    assert!(stdout.contains("native-group-lock-smoke: busy-rejected=true members-only=true pointer-blocked=true keyboard-blocked=true unlocked-edit=true system-input=false"));
    assert!(stdout.contains("native-group-visibility-smoke: members-only=true failure-rollback=true tray-restores-group=true state-restored=true system-tray=false"));
    assert!(stdout.contains("native-group-catalog-smoke: empty-rename=true empty-delete=true empty-close=true unrelated-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-execution-owner-smoke: close-retires=true stale-rejected=true unrelated-preserved=true exit-cleared=true network=false"));
    assert!(stdout.contains("native-default-capture-smoke: new-pin=true modal-disabled=true recovered=true desktop-access=false"));
    assert!(stdout.contains(
        "native-pin-origin-smoke: negative-origin=true physical-size=true desktop-access=false"
    ));
    assert!(stdout.contains("native-new-pin-smoke: worker=true new-window=true shared-pixels=true source-history=true desktop-access=false"));
    assert!(stdout.contains("native-paste-pin-smoke: png=true dib=true file=true source-preserved=true pending-bounded=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-batch-smoke: count=2 source-preserved=true replacement-rejected=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-editable-smoke: independent-units=true fresh-annotations=true crop=true opacity=true undo-isolated=true cancelled=true clipboard-access=false"));
    assert!(stdout.contains("native-paste-capture-smoke: metadata=true unknown-fields=true new-document-binding=true path-access=false"));
    assert!(stdout.contains("native-paste-placement-smoke: physical-size=true cascade=true pending-refresh=true failed-staging=true new-copy-isolated=true clipboard-access=false"));
    assert!(stdout.contains("native-history-delivery-smoke: text=true single-undo=true stale-rejected=true modal-rejected=true independent-pin=true clipboard-access=false"));
    assert!(stdout.contains("native-open-pin-smoke: worker=true new-window=true source-preserved=true cancelled=true failed=true stale-rejected=true dialog-access=false"));
    assert!(stdout
        .contains("native-open-batch-smoke: count=2 source-preserved=true dialog-access=false"));
    assert!(stdout.contains("native-drop-pin-smoke: shell=true worker=true new-window=true source-preserved=true close-independent=true system-input=false"));
    assert!(stdout.contains(
        "native-drop-batch-smoke: count=2 one-worker=true source-preserved=true system-input=false"
    ));
    assert!(stdout.contains("native-import-rollback-smoke: staged-host-released=true staged-panel-released=true collection-unchanged=true"));
    assert!(stdout.contains("native-shared-modal-smoke: capture-blocked=true local-independent=true released=true desktop-access=false"));
    assert!(stdout.contains("native-shared-capture-smoke: windows=4 unique=true hidden-preserved=true shared-gate=true desktop-access=false"));
    assert!(stdout.contains("native-application-hotkey-smoke: shared-menu=true coalesced=true cross-window=true failure-preserved=true system-registration=false"));
    assert!(stdout.contains("native-application-hotkey-secondary-smoke: live=true long=true unique-owner=true cleanup=true system-registration=false"));
    assert!(stdout.contains("native-restored-hotkey-transfer-smoke: live=true long=true rollback=true unique-owner=true system-registration=false"));
    assert!(stdout.contains("native-application-smoke: command=true coalesced=true close-initial=true reopen=true collected=true"));
    assert!(
        stdout.contains("native-document-owner-smoke: relocated=true state-stable=true icons=true")
    );
    assert!(stdout.contains(
        "native-capture-protection-smoke: host=true panel=true reset=true desktop-access=false"
    ));
    assert!(stdout.contains(
        "native-tool-panel-availability-smoke: disabled=true queued-rejected=true recovered=true"
    ));
    assert!(stdout.contains(
        "native-tool-properties-smoke: controls=16 width=true opacity=true fill=true labels=true"
    ));
    assert!(stdout.contains(
        "native-tool-panel-refresh-smoke: unchanged-timers=8 refreshed-after-change=true"
    ));
    assert!(stdout.contains("native-marquee-smoke: pointer=true shift=true capture=true escape=true wheel=true undo=true"));
    assert!(stdout.contains(
        "native-crop-effect-smoke: timer=true atomic-view=true reset=true escape=true window=true flip=true"
    ));
    assert!(stdout.contains("native-flip-smoke: horizontal=true vertical=true timer=true undo=true redo=true escape=true"));
}

#[test]
fn pin_requires_an_image() {
    let output = run_bounded(Command::new(env!("CARGO_BIN_EXE_otter-native")).arg("--pin"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires an image"));
}

#[test]
fn invalid_image_does_not_fall_back_to_fake_success() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let output = run_bounded(
        Command::new(env!("CARGO_BIN_EXE_otter-native"))
            .arg(file)
            .arg("--smoke-test"),
    );
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("native-smoke:"));
}
