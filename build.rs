//! Records the commit the binary is built from as `OBOT_COMMIT`, shown in the bot's status.
//! CI and the Dockerfiles pass it in, since image builds have no `.git`; local builds ask git.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=OBOT_COMMIT");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    let commit = std::env::var("OBOT_COMMIT")
        .ok()
        .filter(|commit| !commit.is_empty())
        .or_else(|| {
            let output = Command::new("git")
                .args(["rev-parse", "--short=7", "HEAD"])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    // Full hashes from CI are shortened like `git rev-parse --short`
    let commit: String = commit.chars().take(7).collect();
    println!("cargo:rustc-env=OBOT_COMMIT={commit}");
}
