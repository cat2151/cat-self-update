use std::process::Command;

const RUN_NETWORK_TESTS_ENV: &str = "RUN_NETWORK_TESTS";

fn app_bin() -> String {
    std::env::var("CARGO_BIN_EXE_cat-self-update").expect("binary path should be set by cargo test")
}

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("app crate should live under the workspace root")
        .to_path_buf()
}

fn remote_main_is_reachable() -> bool {
    let output = git_command_without_prompt()
        .args([
            "ls-remote",
            "https://github.com/cat2151/cat-self-update",
            "refs/heads/main",
        ])
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn git_command_without_prompt() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "");
    command
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
fn help_lists_check_subcommand() {
    let output = Command::new(app_bin())
        .arg("--help")
        .output()
        .expect("help command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("help output should be utf-8");
    assert!(stdout.contains("check"));
    assert!(stdout.contains("Compare"));
    assert!(stdout.contains("commit hash"));
    assert!(stdout.contains("remote"));
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

    if actual_hash == "unknown" {
        eprintln!("hash CLI returned 'unknown'; skipping git hash comparison test");
        return;
    }

    let expected_result = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_root())
        .output();

    let expected = match expected_result {
        Ok(output) => output,
        Err(err) => {
            eprintln!(
                "git not available or failed to start ({}); skipping git hash comparison test",
                err
            );
            return;
        }
    };

    if !expected.status.success() {
        eprintln!(
            "git rev-parse HEAD failed with status {:?}; skipping git hash comparison test",
            expected.status
        );
        return;
    }

    let expected_hash = String::from_utf8(expected.stdout).expect("git hash should be utf-8");
    assert_eq!(actual_hash, expected_hash.trim());
}

#[test]
fn check_prints_embedded_remote_and_result() {
    if std::env::var_os(RUN_NETWORK_TESTS_ENV).is_none() {
        eprintln!(
            "{RUN_NETWORK_TESTS_ENV} is not set; skipping network-dependent check CLI test"
        );
        return;
    }

    if !remote_main_is_reachable() {
        eprintln!("remote main branch not reachable via git ls-remote; skipping check CLI test");
        return;
    }

    let output = Command::new(app_bin())
        .arg("check")
        .output()
        .expect("check command should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("check output should be utf-8");
    assert!(stdout.contains("embedded: "));
    assert!(stdout.contains("remote: "));
    assert!(stdout.contains("result: "));
}
