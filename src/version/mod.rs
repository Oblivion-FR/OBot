//! Which build is running, shown in the bot's status, `/version` and the panel's footer

/// Package version, from `Cargo.toml`
pub const NUMBER: &str = env!("CARGO_PKG_VERSION");

/// Short hash of the commit the binary is built from, `unknown` when build.rs couldn't tell
pub const COMMIT: &str = env!("OBOT_COMMIT");

/// Source repository, from `Cargo.toml`
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// Version and commit, like `0.1.0 (20c7fcf)`
pub fn label() -> String {
    format!("{NUMBER} ({COMMIT})")
}

/// Page of the commit on GitHub, `None` when the commit is unknown
pub fn commit_url() -> Option<String> {
    commit_url_of(COMMIT)
}

fn commit_url_of(commit: &str) -> Option<String> {
    (commit != "unknown").then(|| format!("{REPOSITORY}/commit/{commit}"))
}

#[cfg(test)]
mod tests;
