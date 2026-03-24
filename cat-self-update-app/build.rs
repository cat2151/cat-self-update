fn main() {
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| output.status.success().then_some(output))
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_COMMIT_HASH={hash}");

    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
    {
        if output.status.success() {
            let git_dir = std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            if let Ok(git_dir) = git_dir.canonicalize() {
                let head_path = git_dir.join("HEAD");
                println!("cargo:rerun-if-changed={}", head_path.display());

                if let Ok(head) = std::fs::read_to_string(&head_path) {
                    if let Some(ref_path) = head.trim().strip_prefix("ref: ") {
                        let ref_path = std::path::Path::new(ref_path);
                        if !ref_path.as_os_str().is_empty()
                            && ref_path.components().all(|component| {
                                matches!(component, std::path::Component::Normal(_))
                            })
                        {
                            let ref_watch_path = git_dir.join(ref_path);
                            if let Ok(ref_watch_path) = ref_watch_path.canonicalize() {
                                if ref_watch_path.starts_with(&git_dir) {
                                    println!("cargo:rerun-if-changed={}", ref_watch_path.display());
                                }
                            }
                        }
                    }
                }

                println!(
                    "cargo:rerun-if-changed={}",
                    git_dir.join("packed-refs").display()
                );
            }
        }
    }
}
