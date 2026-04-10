use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub embedded_hash: String,
    pub remote_hash: String,
}

impl CheckResult {
    pub fn is_up_to_date(&self) -> bool {
        self.embedded_hash == self.remote_hash
    }
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = if self.is_up_to_date() {
            "up-to-date"
        } else {
            "update available"
        };

        write!(
            f,
            "embedded: {}\nremote: {}\nresult: {}",
            self.embedded_hash, self.remote_hash, result
        )
    }
}

pub fn compare_hashes(embedded_hash: &str, remote_hash: &str) -> CheckResult {
    CheckResult {
        embedded_hash: embedded_hash.to_string(),
        remote_hash: remote_hash.to_string(),
    }
}

pub fn check_remote_commit(
    owner: &str,
    repo: &str,
    branch: &str,
    embedded_hash: &str,
) -> Result<CheckResult, Box<dyn std::error::Error>> {
    let remote_hash = fetch_remote_branch_head(owner, repo, branch)?;
    Ok(compare_hashes(embedded_hash, &remote_hash))
}

/// Initiates a self-update by generating a Python helper script in the system
/// temp directory and launching it asynchronously in a separate process.
///
/// # Arguments
/// * `owner` – GitHub repository owner (e.g. `"cat2151"`)
/// * `repo`  – GitHub repository name (e.g. `"cat-self-update"`)
/// * `packages` – package names to pass to `cargo install`, then launch after
///   installation. When empty, installation uses Cargo's default package
///   selection and the repository name is used as the binary name to launch.
pub fn self_update(
    owner: &str,
    repo: &str,
    packages: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let py_content = generate_py_script(owner, repo, packages, std::process::id());
    let py_path = unique_tmp_path();

    fs::write(&py_path, &py_content)?;

    spawn_python(&py_path)?;

    Ok(())
}

/// Escape a string so it can be safely embedded inside a single-quoted
/// Python string literal. Escapes backslashes and single quotes.
fn escape_py_single_quoted(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

/// Build the Python script that will be written to a temp file.
fn generate_py_script(owner: &str, repo: &str, packages: &[&str], parent_pid: u32) -> String {
    let repo_url = format!("https://github.com/{}/{}", owner, repo);
    let repo_url_escaped = escape_py_single_quoted(&repo_url);

    // Build the cargo install command as a Python list literal.
    let install_parts = if packages.is_empty() {
        format!(
            "['cargo', 'install', '--force', '--git', '{}']",
            repo_url_escaped
        )
    } else {
        let package_args = packages
            .iter()
            .map(|package| format!("'{}'", escape_py_single_quoted(package)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "['cargo', 'install', '--force', '--git', '{}', {}]",
            repo_url_escaped, package_args
        )
    };

    // Determine which binary (or binaries) to launch after install.
    let launch_stmts: String = if packages.is_empty() {
        let repo_escaped = escape_py_single_quoted(repo);
        format!("    launch(['{}'])\n", repo_escaped)
    } else {
        packages
            .iter()
            .map(|package| {
                let package_escaped = escape_py_single_quoted(package);
                format!("    launch(['{}'])\n", package_escaped)
            })
            .collect()
    };

    format!(
        r#"import subprocess
import os
import shlex
import sys
import traceback

PARENT_PID = {parent_pid}
INSTALL_PARTS = {install_parts}

def log(message):
    print(message, flush=True)

def format_command(parts):
    if sys.platform == 'win32':
        return subprocess.list2cmdline(parts)
    return shlex.join(parts)

def wait_for_parent_exit():
    if sys.platform != 'win32':
        return

    import ctypes

    SYNCHRONIZE = 0x00100000
    INFINITE = 0xFFFFFFFF
    kernel32 = ctypes.windll.kernel32
    handle = kernel32.OpenProcess(SYNCHRONIZE, False, PARENT_PID)
    if not handle:
        return

    try:
        kernel32.WaitForSingleObject(handle, INFINITE)
    finally:
        kernel32.CloseHandle(handle)

def launch(parts):
    log(f"起動しています: {{format_command(parts)}}")
    subprocess.Popen(parts)

def wait_for_user_acknowledgement():
    if sys.platform != 'win32':
        return

    log("Enterキーを押すと閉じます")
    try:
        input()
    except EOFError:
        pass

try:
    log("現在のプロセスの終了を待っています")
    wait_for_parent_exit()
    log("cargo installを起動しています")
    log(f"$ {{format_command(INSTALL_PARTS)}}")
    subprocess.run(INSTALL_PARTS, check=True)
    log("cargo install が完了しました")
{launch_stmts}
except subprocess.CalledProcessError as err:
    log(f"更新に失敗しました。終了コード: {{err.returncode}}")
    wait_for_user_acknowledgement()
    sys.exit(err.returncode)
except Exception as err:
    log(f"更新処理に失敗しました: {{err}}")
    traceback.print_exc()
    wait_for_user_acknowledgement()
    sys.exit(1)
finally:
    try:
        os.remove(__file__)
    except OSError:
        pass
"#,
        parent_pid = parent_pid,
        install_parts = install_parts,
        launch_stmts = launch_stmts,
    )
}

fn fetch_remote_branch_head(
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let repo_url = format!("https://github.com/{owner}/{repo}");
    let ref_name = format!("refs/heads/{branch}");
    let output = git_command_without_prompt()
        .args(["ls-remote", repo_url.as_str(), ref_name.as_str()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git ls-remote failed with status {}", output.status)
        } else {
            format!("git ls-remote failed: {stderr}")
        };
        return Err(message.into());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("git ls-remote returned invalid UTF-8 output: {err}"))?;
    parse_ls_remote_hash(&stdout, &ref_name)
        .ok_or_else(|| format!("could not find remote hash for {ref_name}").into())
}

fn git_command_without_prompt() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "");
    command
}

fn parse_ls_remote_hash(output: &str, ref_name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if name != ref_name {
            return None;
        }

        if parts.next().is_some() {
            return None;
        }

        Some(hash.to_string())
    })
}

/// Return a path inside the system temp directory that is unique for this
/// process invocation.
fn unique_tmp_path() -> PathBuf {
    let pid = std::process::id();
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("cat_self_update_{}_{}.py", pid, timestamp_nanos);
    std::env::temp_dir().join(filename)
}

/// Spawn the Python interpreter with the given script path.
/// On Windows the process is started in a new console so progress is visible
/// while the parent executable exits and releases its file lock.
fn spawn_python(py_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new("python")
            .arg(py_path)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()?;
    }

    #[cfg(not(windows))]
    {
        Command::new("python3").arg(py_path).spawn()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_python_script_has_valid_syntax(script: &str) {
        let script_path = unique_tmp_path();
        fs::write(&script_path, script).expect("should write generated script to a temp file");

        let compile_command =
            "import pathlib, sys; compile(pathlib.Path(sys.argv[1]).read_text(encoding='utf-8'), sys.argv[1], 'exec')";
        let python_candidates: &[(&str, &[&str])] = if cfg!(windows) {
            &[("python", &[]), ("py", &["-3"])]
        } else {
            &[("python3", &[]), ("python", &[])]
        };

        let mut output = None;
        for (program, args) in python_candidates {
            match Command::new(program)
                .args(*args)
                .arg("-c")
                .arg(compile_command)
                .arg(&script_path)
                .output()
            {
                Ok(candidate_output) => {
                    output = Some((program, *args, candidate_output));
                    break;
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => panic!("failed to run {program}: {err}"),
            }
        }

        let (program, args, output) = output
            .unwrap_or_else(|| panic!("could not find a Python interpreter for syntax validation"));
        let command = std::iter::once((*program).to_string())
            .chain(args.iter().map(|arg| (*arg).to_string()))
            .chain(["-c".to_string(), compile_command.to_string()])
            .collect::<Vec<_>>()
            .join(" ");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        fs::remove_file(&script_path).ok();

        assert!(
            output.status.success(),
            "generated Python script has invalid syntax\ncommand: {command}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn py_script_contains_repo_url() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("https://github.com/cat2151/cat-self-update"));
    }

    #[test]
    fn py_script_contains_cargo_install() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("cargo"));
        assert!(script.contains("install"));
        assert!(script.contains("--force"));
        assert!(script.contains("--git"));
    }

    #[test]
    fn py_script_installs_specified_packages() {
        let script = generate_py_script("owner", "repo", &["my-bin", "other-bin"], 1234);
        assert!(script.contains(
            "INSTALL_PARTS = ['cargo', 'install', '--force', '--git', 'https://github.com/owner/repo', 'my-bin', 'other-bin']"
        ));
    }

    #[test]
    fn py_script_logs_install_progress_and_command() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("cargo installを起動しています"));
        assert!(script.contains("format_command(INSTALL_PARTS)"));
        assert!(script.contains("subprocess.run(INSTALL_PARTS, check=True)"));
    }

    #[test]
    fn py_script_waits_for_parent_exit_on_windows() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("PARENT_PID = 1234"));
        assert!(script.contains("OpenProcess"));
        assert!(script.contains("WaitForSingleObject"));
    }

    #[test]
    fn py_script_does_not_hide_cargo_output() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(!script.contains("subprocess.DEVNULL"));
        assert!(!script.contains("creationflags"));
    }

    #[test]
    fn py_script_launches_repo_binary_when_packages_empty() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("    launch(['cat-self-update'])"));
    }

    #[test]
    fn py_script_launches_specified_packages() {
        let script = generate_py_script("owner", "repo", &["my-bin", "other-bin"], 1234);
        assert!(script.contains("    launch(['my-bin'])"));
        assert!(script.contains("    launch(['other-bin'])"));
    }

    #[test]
    fn py_script_has_valid_python_syntax_when_launching_repo_binary() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert_python_script_has_valid_syntax(&script);
    }

    #[test]
    fn py_script_has_valid_python_syntax_when_launching_multiple_packages() {
        let script = generate_py_script("owner", "repo", &["my-bin", "other-bin"], 1234);
        assert_python_script_has_valid_syntax(&script);
    }

    #[test]
    fn py_script_removes_itself() {
        let script = generate_py_script("cat2151", "cat-self-update", &[], 1234);
        assert!(script.contains("os.remove(__file__)"));
        assert!(script.contains("except OSError"));
    }

    #[test]
    fn py_script_escapes_single_quotes() {
        let script = generate_py_script("o\\'wner", "re\\'po", &["bi\\'n"], 1234);
        assert!(!script.contains("o\\'wner") || script.contains("o\\\\'wner"));
        // Ensure the escape helper works directly
        assert_eq!(escape_py_single_quoted("a'b"), "a\\'b");
        assert_eq!(escape_py_single_quoted("a\\b"), "a\\\\b");
    }

    #[test]
    fn unique_tmp_path_is_in_temp_dir() {
        let path = unique_tmp_path();
        assert!(path.starts_with(std::env::temp_dir()));
    }

    #[test]
    fn unique_tmp_path_has_expected_prefix() {
        let path = unique_tmp_path();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("cat_self_update_"));
        assert!(name.ends_with(".py"));
    }

    #[test]
    fn compare_hashes_reports_up_to_date() {
        let result = compare_hashes("abc", "abc");
        assert!(result.is_up_to_date());
        assert_eq!(
            result.to_string(),
            "embedded: abc\nremote: abc\nresult: up-to-date"
        );
    }

    #[test]
    fn compare_hashes_reports_update_available() {
        let result = compare_hashes("abc", "def");
        assert!(!result.is_up_to_date());
        assert_eq!(
            result.to_string(),
            "embedded: abc\nremote: def\nresult: update available"
        );
    }

    #[test]
    fn parse_ls_remote_hash_finds_matching_branch() {
        let output = "abc123\trefs/heads/main\ndef456\trefs/heads/feature\n";
        assert_eq!(
            parse_ls_remote_hash(output, "refs/heads/main"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn parse_ls_remote_hash_returns_none_for_missing_branch() {
        let output = "abc123\trefs/heads/main\n";
        assert_eq!(parse_ls_remote_hash(output, "refs/heads/release"), None);
    }

    #[test]
    fn parse_ls_remote_hash_rejects_extra_fields() {
        let output = "abc123\trefs/heads/main\textra\n";
        assert_eq!(parse_ls_remote_hash(output, "refs/heads/main"), None);
    }
}
