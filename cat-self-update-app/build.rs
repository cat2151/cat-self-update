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
            let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !git_dir.is_empty() {
                println!("cargo:rerun-if-changed={git_dir}/HEAD");

                if let Ok(head) = std::fs::read_to_string(format!("{git_dir}/HEAD")) {
                    if let Some(ref_path) = head.trim().strip_prefix("ref: ") {
                        println!("cargo:rerun-if-changed={git_dir}/{ref_path}");
                    }
                }

                println!("cargo:rerun-if-changed={git_dir}/packed-refs");
            }
        }
    }
}
