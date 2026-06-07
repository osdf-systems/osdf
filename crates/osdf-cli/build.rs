fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-env-changed=OSDF_AUTO_INSTALL");

    #[cfg(windows)]
    if should_auto_install() {
        auto_install::spawn();
    }
}

#[cfg(windows)]
fn should_auto_install() -> bool {
    std::env::var("OSDF_AUTO_INSTALL").ok().as_deref() == Some("1")
        && std::env::var("PROFILE").ok().as_deref() == Some("release")
}

#[cfg(windows)]
mod auto_install {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    pub fn spawn() {
        let manifest_dir = PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"),
        );
        let Some(workspace_root) = manifest_dir.parent().and_then(|path| path.parent()) else {
            return;
        };

        let target_dir = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace_root.join("target"));
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
        let binary = target_dir.join(profile).join("osdf.exe");
        let install_script = workspace_root.join("scripts").join("install-cli.ps1");

        if !install_script.is_file() {
            return;
        }

        std::thread::spawn(move || {
            if !wait_for_stable_binary(&binary, Duration::from_secs(180)) {
                eprintln!(
                    "osdf: auto-install skipped (binary not ready): {}",
                    binary.display()
                );
                return;
            }

            let status = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &install_script.to_string_lossy(),
                    "-CopyOnly",
                    "-BinaryPath",
                    &binary.to_string_lossy(),
                    "-SkipPathUpdate",
                ])
                .status();

            match status {
                Ok(result) if result.success() => {
                    eprintln!("osdf: updated {}", binary.display());
                }
                Ok(result) => {
                    eprintln!(
                        "osdf: auto-install failed with exit code {:?}",
                        result.code()
                    );
                }
                Err(err) => {
                    eprintln!("osdf: auto-install error: {err}");
                }
            }
        });
    }

    fn wait_for_stable_binary(path: &Path, timeout: Duration) -> bool {
        let started = Instant::now();
        let mut last_len = None;
        let mut stable_since = None;

        while started.elapsed() < timeout {
            if let Ok(metadata) = std::fs::metadata(path) {
                let len = metadata.len();
                if len > 0 {
                    if last_len == Some(len) {
                        if stable_since
                            .map(|since: Instant| since.elapsed() >= Duration::from_millis(400))
                            .unwrap_or(false)
                        {
                            return true;
                        }
                    } else {
                        last_len = Some(len);
                        stable_since = Some(Instant::now());
                    }
                }
            } else {
                last_len = None;
                stable_since = None;
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        false
    }
}
