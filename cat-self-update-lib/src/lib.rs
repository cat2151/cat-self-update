use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Build the Python script that will be written to a temp file.
fn generate_py_script(owner: &str, repo: &str, bins: &[&str]) -> String {
    let repo_url = format!("https://github.com/{}/{}", owner, repo);

    // Build the cargo install command as a Python list literal.
    let install_parts = format!(
        "['cargo', 'install', '--force', '--git', '{}']",
        repo_url
    );

    // Determine which binary (or binaries) to launch after install.
    let launch_stmts: String = if bins.is_empty() {
        // Default: launch the binary that matches the repository name.
        format!("subprocess.Popen(['{}'], **popen_kwargs)\n", repo)
    } else {
        bins.iter()
            .map(|b| format!("subprocess.Popen(['{}'], **popen_kwargs)\n", b))
            .collect()
    };

    format!(
        r#"import subprocess
import os
import sys

if sys.platform == 'win32':
    DETACHED_PROCESS = 0x00000008
    popen_kwargs = {{'creationflags': DETACHED_PROCESS}}
else:
    popen_kwargs = {{}}

subprocess.run({install_parts}, check=True)

{launch_stmts}
os.remove(__file__)
"#,
        install_parts = install_parts,
        launch_stmts = launch_stmts,
    )
}

/// Return a path inside the system temp directory that is unique for this
/// process invocation.
fn unique_tmp_path() -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let filename = format!("cat_self_update_{}_{}.py", pid, nanos);
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
}
