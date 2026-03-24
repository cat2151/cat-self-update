use std::fs;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::process::Stdio;

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
/// temp directory and launching it asynchronously (detached from the current
/// process).
///
/// # Arguments
/// * `owner` – GitHub repository owner (e.g. `"cat2151"`)
/// * `repo`  – GitHub repository name (e.g. `"cat-self-update"`)
/// * `bins`  – additional binary names to launch after installation; when
///   empty the repository name itself is used as the binary name
pub fn self_update(
    owner: &str,
    repo: &str,
    bins: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let py_content = generate_py_script(owner, repo, bins);
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
fn generate_py_script(owner: &str, repo: &str, bins: &[&str]) -> String {
    let repo_url = format!("https://github.com/{}/{}", owner, repo);
    let repo_url_escaped = escape_py_single_quoted(&repo_url);

    // Build the cargo install command as a Python list literal.
    let install_parts = format!(
        "['cargo', 'install', '--force', '--git', '{}']",
        repo_url_escaped
    );

    // Determine which binary (or binaries) to launch after install.
    let launch_stmts: String = if bins.is_empty() {
        let repo_escaped = escape_py_single_quoted(repo);
        format!("subprocess.Popen(['{}'], **popen_kwargs)\n", repo_escaped)
    } else {
        bins.iter()
            .map(|b| {
                let b_escaped = escape_py_single_quoted(b);
                format!("subprocess.Popen(['{}'], **popen_kwargs)\n", b_escaped)
            })
            .collect()
    };

    format!(
        r#"import subprocess
import os
import sys

if sys.platform == 'win32':
    DETACHED_PROCESS = 0x00000008
    popen_kwargs = {{
        'creationflags': DETACHED_PROCESS,
        'stdin': subprocess.DEVNULL,
        'stdout': subprocess.DEVNULL,
        'stderr': subprocess.DEVNULL,
    }}
else:
    popen_kwargs = {{}}

subprocess.run({install_parts}, check=True, **popen_kwargs)

{launch_stmts}
try:
    os.remove(__file__)
except OSError:
    pass
"#,
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
    let output = Command::new("git")
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

    let stdout = String::from_utf8(output.stdout)?;
    parse_ls_remote_hash(&stdout, &ref_name)
        .ok_or_else(|| format!("could not find remote hash for {ref_name}").into())
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
/// On Windows the process is started as DETACHED_PROCESS so it outlives the
/// parent executable.
fn spawn_python(py_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        Command::new("python")
            .arg(py_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS)
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

    #[test]
    fn py_script_contains_repo_url() {
        let script = generate_py_script("cat2151", "cat-self-update", &[]);
        assert!(script.contains("https://github.com/cat2151/cat-self-update"));
    }

    #[test]
    fn py_script_contains_cargo_install() {
        let script = generate_py_script("cat2151", "cat-self-update", &[]);
        assert!(script.contains("cargo"));
        assert!(script.contains("install"));
        assert!(script.contains("--force"));
        assert!(script.contains("--git"));
    }

    #[test]
    fn py_script_redirects_windows_stdio_to_devnull() {
        let script = generate_py_script("cat2151", "cat-self-update", &[]);
        assert!(script.contains("'stdin': subprocess.DEVNULL"));
        assert!(script.contains("'stdout': subprocess.DEVNULL"));
        assert!(script.contains("'stderr': subprocess.DEVNULL"));
        assert!(script.contains("subprocess.run("));
        assert!(script.contains("check=True, **popen_kwargs"));
    }

    #[test]
    fn py_script_launches_repo_binary_when_bins_empty() {
        let script = generate_py_script("cat2151", "cat-self-update", &[]);
        assert!(script.contains("'cat-self-update'"));
    }

    #[test]
    fn py_script_launches_specified_bins() {
        let script = generate_py_script("owner", "repo", &["my-bin", "other-bin"]);
        assert!(script.contains("'my-bin'"));
        assert!(script.contains("'other-bin'"));
    }

    #[test]
    fn py_script_removes_itself() {
        let script = generate_py_script("cat2151", "cat-self-update", &[]);
        assert!(script.contains("os.remove(__file__)"));
        assert!(script.contains("except OSError"));
    }

    #[test]
    fn py_script_escapes_single_quotes() {
        let script = generate_py_script("o\\'wner", "re\\'po", &["bi\\'n"]);
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
        assert_eq!(result.to_string(), "embedded: abc\nremote: abc\nresult: up-to-date");
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
}
