use super::*;
use base64::Engine;

fn pem() -> String {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
        base64::engine::general_purpose::STANDARD.encode(certificate.cert.der())
    )
}

#[test]
fn public_root_snapshot_matches_pinned_baseline_and_keeps_scoped_ca_separate() {
    let roots = Trust::public_roots();
    assert!(!roots.is_empty());
    assert_eq!(roots.len(), webpki_roots::TLS_SERVER_ROOTS.len());
    let baseline = roots.len();
    let scoped = Scoped::from_pem("https://localhost", pem().as_bytes()).unwrap();
    let trust = Trust::new(roots, Some(scoped));
    assert_eq!(
        trust.roots_for("https://example.com").unwrap().len(),
        baseline
    );
    assert_eq!(
        trust.roots_for("https://localhost").unwrap().len(),
        baseline + 1
    );
}

#[test]
fn certificate_only_pem_is_bounded_and_fingerprinted() {
    let pem = pem();
    let trust = Scoped::from_pem("https://localhost", pem.as_bytes()).unwrap();
    assert_eq!(
        trust.fingerprint(),
        <[u8; 32]>::from(Sha256::digest(pem.as_bytes()))
    );
    assert_eq!(trust.roots.len(), 1);
    assert!(Scoped::from_pem("https://localhost", pem.repeat(8).as_bytes()).is_ok());
    assert!(matches!(
        Scoped::from_pem("https://localhost", pem.repeat(9).as_bytes()),
        Err(Error::Limit)
    ));
    for bytes in [
        b"".as_slice(),
        b" \n",
        b"\xff",
        b"-----BEGIN PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----",
        b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----",
    ] {
        assert!(Scoped::from_pem("https://localhost", bytes).is_err());
    }
    assert!(Scoped::from_pem("https://localhost", format!("{pem}garbage").as_bytes()).is_err());
    assert!(matches!(
        Scoped::from_pem("https://localhost", &vec![b' '; MAX_BYTES + 1]),
        Err(Error::Limit)
    ));
}

#[test]
fn private_roots_are_exact_origin_scoped_and_defaults_are_retained() {
    let pem = pem();
    let scoped = Scoped::from_pem("https://LOCALHOST:443/", pem.as_bytes()).unwrap();
    let mut defaults = RootCertStore::empty();
    defaults
        .add(CertificateDer::from_pem_slice(pem.as_bytes()).unwrap())
        .unwrap();
    let trust = Trust::new(defaults, Some(scoped));
    assert_eq!(trust.roots_for("https://localhost").unwrap().len(), 2);
    for other in [
        "https://localhost:444",
        "https://other.example",
        "http://localhost",
    ] {
        assert_eq!(trust.roots_for(other).unwrap().len(), 1);
    }
    for bad in [
        "http://localhost",
        "https://user@localhost",
        "https://localhost/path",
        "https://localhost?q=x",
        "https://localhost#fragment",
        " https://localhost",
    ] {
        assert!(Scoped::from_pem(bad, pem.as_bytes()).is_err());
    }
    assert!(trust.roots_for("https://localhost/path").is_err());
}

#[test]
fn canonical_environment_takes_precedence_and_loader_is_bounded() {
    let path = std::env::current_dir()
        .unwrap()
        .join("operator-ca-fixture.pem");
    let pem = pem();
    let lookup = |key: &str| match key {
        "OTTER_LOOM_TLS_ORIGIN" => Some(OsString::from("https://localhost")),
        "OTTER_LOOM_TLS_CA_FILE" => Some(path.clone().into_os_string()),
        "HOOK_LOOM_TLS_ORIGIN" => Some(OsString::from("https://wrong.example")),
        _ => None,
    };
    let trust = Trust::load_with(RootCertStore::empty(), lookup, |actual| {
        assert_eq!(actual, path);
        Ok(io::Cursor::new(pem.as_bytes()))
    })
    .unwrap();
    assert_eq!(trust.roots_for("https://localhost").unwrap().len(), 1);
    assert!(trust.roots_for("https://wrong.example").unwrap().is_empty());
    assert!(matches!(
        Trust::load_with(RootCertStore::empty(), lookup, |_| Ok(io::repeat(b' '))),
        Err(Error::Limit)
    ));
    assert!(matches!(
        Trust::load_with(RootCertStore::empty(), lookup, |_| Err::<io::Empty, _>(
            io::ErrorKind::PermissionDenied.into()
        )),
        Err(Error::Read)
    ));
}

#[test]
fn incomplete_empty_relative_and_invalid_config_never_open_file() {
    let path = std::env::current_dir()
        .unwrap()
        .join("operator-ca-fixture.pem");
    for (origin, file) in [
        (None, Some(path.as_os_str())),
        (Some("https://localhost"), None),
        (Some(""), Some(path.as_os_str())),
        (Some("http://localhost"), Some(path.as_os_str())),
        (
            Some("https://localhost"),
            Some(std::ffi::OsStr::new("relative.pem")),
        ),
    ] {
        let result = Trust::load_with(
            RootCertStore::empty(),
            |key| match key {
                "OTTER_LOOM_TLS_ORIGIN" => origin.map(OsString::from),
                "OTTER_LOOM_TLS_CA_FILE" => file.map(OsString::from),
                // 空规范 origin 不能退回这个有效值。
                "HOOK_LOOM_TLS_ORIGIN" if origin == Some("") => {
                    Some(OsString::from("https://localhost"))
                }
                _ => None,
            },
            |_| -> io::Result<io::Empty> { panic!("invalid configuration opened a file") },
        );
        assert!(result.is_err());
    }
    let trust = Trust::load_with(
        RootCertStore::empty(),
        |_| None,
        |_| -> io::Result<io::Empty> { panic!("unset configuration opened a file") },
    )
    .unwrap();
    assert!(trust.roots_for("https://example.com").unwrap().is_empty());
}
