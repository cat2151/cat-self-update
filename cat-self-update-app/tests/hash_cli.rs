use std::process::Command;

fn app_bin() -> String {
    std::env::var("CARGO_BIN_EXE_cat-self-update").expect("binary path should be set by cargo test")
}

#[test]
fn help_lists_hash_subcommand() {
    let output = Command::new(app_bin())
        .arg("--help")
        .output()
        .expect("help command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be utf-8");
    assert!(stdout.contains("hash"));
    assert!(stdout.contains("Print the build-time commit hash"));
}

#[test]
fn hash_prints_embedded_head_commit() {
    let output = Command::new(app_bin())
        .arg("hash")
        .output()
        .expect("hash command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("hash output should be utf-8");
    let actual_hash = stdout.trim();
    assert!(!actual_hash.is_empty());
    assert_ne!(actual_hash, "unknown");

    let expected = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir("/home/runner/work/cat-self-update/cat-self-update")
        .output()
        .expect("git rev-parse should run");
    assert!(expected.status.success());

    let expected_hash = String::from_utf8(expected.stdout).expect("git hash should be utf-8");
    assert_eq!(actual_hash, expected_hash.trim());
}
