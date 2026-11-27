//! Real receiving process, isolated profile. No UAC prompt, task change,
//! input desktop interaction, or authority derived from a self-supplied hash.
use super::{process, Fixture, SESSION};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::windows::fs::OpenOptionsExt,
    path::Path,
    process::{Command, Output},
    time::Duration,
};

fn probe(root: Option<&Path>, revision: &str, digest: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_otter-native"));
    cmd.args(["--check-restart-checkpoint", revision, digest])
        .env_remove("OTTER_NATIVE_PROFILE_DIR");
    if let Some(path) = root {
        cmd.env("OTTER_NATIVE_PROFILE_DIR", path);
    }
    process::run(&mut cmd, Duration::from_secs(30)).unwrap()
}
fn rejected(output: Output, why: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(why), "{stderr}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("native-restart-checkpoint:"));
}

#[test]
fn restart_checkpoint_requires_release_then_restores_only_exact_committed_bytes_and_revision() {
    let f = Fixture::new();
    fs::write(f.path(), SESSION).unwrap();
    let hash = format!("{:x}", Sha256::digest(SESSION));
    let lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(f.0.join("workspace.lock"))
        .unwrap();
    rejected(
        probe(Some(&f.0), "7", &hash),
        "Cannot open workspace profile",
    );
    drop(lock);
    let success = probe(Some(&f.0), "7", &hash);
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let stdout = String::from_utf8(success.stdout).unwrap();
    assert!(stdout.contains("native-session-render-check: paints=2 device-recreated=true"));
    assert!(
        stdout.contains("hidden=true")
            && stdout.contains("revision=7 restored=true read-only=true")
    );
    rejected(probe(Some(&f.0), "8", &hash), "published revision");
    let mut edited = SESSION.to_vec();
    edited.push(b' '); // Same valid JSON and revision, distinct publication.
    fs::write(f.path(), &edited).unwrap();
    rejected(probe(Some(&f.0), "7", &hash), "published digest");
    assert_eq!(fs::read(f.path()).unwrap(), edited);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 2);
}

#[test]
fn restart_checkpoint_never_creates_a_missing_profile_lock_or_uses_readonly_recovery() {
    let f = Fixture::new();
    let hash = format!("{:x}", Sha256::digest(SESSION));
    rejected(probe(None, "7", &hash), "requires OTTER_NATIVE_PROFILE_DIR");
    let absent = f.0.join("absent");
    rejected(
        probe(Some(&absent), "7", &hash),
        "Cannot open workspace profile",
    );
    assert!(!absent.exists());
    let recovery =
        f.0.join("workspace.json.otter-recovery-00000000-0000-0000-0000-000000000001.json");
    fs::write(&recovery, SESSION).unwrap();
    rejected(
        probe(Some(&f.0), "7", &hash),
        "Cannot open workspace profile",
    );
    let lock = f.0.join("workspace.lock");
    assert!(!lock.exists());
    fs::write(&lock, b"").unwrap();
    rejected(
        probe(Some(&f.0), "7", &hash),
        "Cannot open workspace profile",
    );
    fs::hard_link(&recovery, f.path()).unwrap();
    rejected(
        probe(Some(&f.0), "7", &hash),
        "Cannot open workspace profile",
    );
    for (rev, digest) in [
        ("07", hash.as_str()),
        ("+7", hash.as_str()),
        ("7", "malformed"),
    ] {
        rejected(probe(Some(&f.0), rev, digest), "checkpoint");
    }
    assert_eq!(fs::read(&recovery).unwrap(), SESSION);
}
