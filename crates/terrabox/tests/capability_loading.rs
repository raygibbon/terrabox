//! TERM failures are tested in child processes without racing environment updates.
#![cfg(unix)]
use std::{io, process::Command};
use terrabox::Terminal;

#[test]
fn environment_failure_child() {
    let Ok(expected) = std::env::var("TERRABOX_CAPABILITY_FAILURE") else {
        return;
    };
    let error = Terminal::new()
        .err()
        .expect("TERM failure must precede backend acquisition");
    let kind = match expected.as_str() {
        "missing" => io::ErrorKind::NotFound,
        "invalid" => io::ErrorKind::InvalidInput,
        "unknown" => io::ErrorKind::Unsupported,
        _ => panic!("unknown test mode"),
    };
    assert_eq!(error.kind(), kind, "{error}");
}

#[test]
fn missing_invalid_and_unknown_term_fail_before_terminal_initialization() {
    for (mode, term) in [
        ("missing", None),
        ("missing", Some("")),
        ("invalid", Some("../not-a-terminal")),
        ("unknown", Some("terra-audit-unknown-terminal")),
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "environment_failure_child", "--nocapture"])
            .env("TERRABOX_CAPABILITY_FAILURE", mode);
        if let Some(term) = term {
            child.env("TERM", term);
        } else {
            child.env_remove("TERM");
        }
        let result = child.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
    use std::os::unix::ffi::OsStringExt;
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "environment_failure_child", "--nocapture"])
        .env("TERRABOX_CAPABILITY_FAILURE", "invalid")
        .env("TERM", std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
}
