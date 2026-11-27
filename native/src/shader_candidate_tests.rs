//! Verify pin/ACL/handle gates only in self-created linshi paths; do not load DLLs or change existing directory permissions.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
};
use windows::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = crate::test_temp_root();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = root.join(format!("otter-shader-candidate-{id:?}"));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn acl(&self, grant: &str) {
        let status = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference='Stop'
$user=[Security.Principal.WindowsIdentity]::GetCurrent().User
$acl=Get-Acl -LiteralPath $env:OTTER_CANDIDATE_TEST_PATH
$acl.SetAccessRuleProtection($true,$false)
foreach($rule in @($acl.Access)){ [void]$acl.RemoveAccessRuleSpecific($rule) }
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user,'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
if($env:OTTER_CANDIDATE_TEST_GRANT){ $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new('S-1-1-0'),$env:OTTER_CANDIDATE_TEST_GRANT,'ContainerInherit,ObjectInherit','None','Allow')) }
Set-Acl -LiteralPath $env:OTTER_CANDIDATE_TEST_PATH -AclObject $acl
"#]).env("OTTER_CANDIDATE_TEST_PATH", &self.0).env("OTTER_CANDIDATE_TEST_GRANT", grant).status().unwrap();
        assert!(status.success());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(files) = fs::read_dir(&self.0) {
            for file in files.flatten() {
                let _ = fs::remove_file(file.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
    }
}
#[test]
fn shader_candidate_rejects_nonlocal_ambiguous_and_device_paths_before_open() {
    for path in [
        "relative",
        "C:relative",
        "C:/",
        r"\\host\share\bundle",
        r"\\?\C:\bundle",
        "Z:/bundle",
        "C:/a/../b",
        "C:/a/./b",
        "C:/a:b",
        "C:/NUL",
        "C:/CON .dll",
        "C:/tail.",
        "C:/a\0",
    ] {
        assert!(
            files::directories(Path::new(path), |_, _| Ok(())).is_err(),
            "{path:?}"
        );
    }
}
#[test]
fn shader_candidate_directory_lease_excludes_rename_not_attribute_handles() {
    let fixture = Fixture::new();
    let leases = files::directory(&fixture.0).unwrap();
    assert!(fs::rename(&fixture.0, fixture.0.with_extension("moved")).is_err());
    assert!(OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(&fixture.0)
        .is_err());
    // Metadata access is not subject to ordinary write-sharing denial; safety must rely on ancestor ACLs rather than this handle.
    assert!(OpenOptions::new()
        .access_mode(0x100)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(&fixture.0)
        .is_ok());
    drop(leases);
    let moved = fixture.0.with_extension("moved");
    fs::rename(&fixture.0, &moved).unwrap();
    fs::rename(&moved, &fixture.0).unwrap();
}
#[test]
fn shader_candidate_acl_rejects_other_user_writes_and_accepts_readonly_grants() {
    for grant in [
        "Modify",
        "WriteAttributes",
        "ChangePermissions",
        "ReadAndExecute",
        "",
    ] {
        // Create each ACL case independently; system updates to existing protected security descriptors are not this module's contract.
        let fixture = Fixture::new();
        fixture.acl(grant);
        let leases = files::directory(&fixture.0).unwrap();
        let accepted = matches!(grant, "ReadAndExecute" | "");
        assert_eq!(acl::check(&leases).is_ok(), accepted, "{grant}");
        assert_eq!(acl::ancestor(&leases).is_ok(), accepted, "{grant}");
    }
}
#[test]
fn shader_candidate_same_handle_hash_lock_hardlink_and_exact_set_gates() {
    let fixture = Fixture::new();
    fixture.acl("");
    let path = fixture.0.join("libEGL.dll");
    fs::write(&path, b"candidate").unwrap();
    let mut file = files::binary(&path).unwrap();
    acl::check(&file).unwrap();
    verify(
        &mut file,
        Fingerprint {
            bytes: 9,
            sha256: Sha256::digest(b"candidate").into(),
        },
    )
    .unwrap();
    assert!(OpenOptions::new().write(true).open(&path).is_err());
    assert!(fs::rename(&path, fixture.0.join("moved.dll")).is_err());
    drop(file);
    fs::hard_link(&path, fixture.0.join("link.dll")).unwrap();
    assert!(files::binary(&path).is_err());
    fs::remove_file(fixture.0.join("link.dll")).unwrap();
    assert!(open(&fixture.0).is_err());
    fs::write(fixture.0.join("libGLESv2.dll"), b"candidate").unwrap();
    assert!(open(&fixture.0).is_err());
    fs::write(fixture.0.join("extra.dll"), b"candidate").unwrap();
    assert!(open(&fixture.0).is_err());
}
