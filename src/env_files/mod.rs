//! Environment files, following the dotenv-flow convention of Next.js, Vite or Rails: settings
//! shared by every environment in `.env`, overridden per environment in `.env.<mode>`, and per
//! machine in the `.local` variants.

use std::path::Path;

use crate::Error;

/// Picks the environment: `OBOT_ENV` when set, otherwise `development` for debug builds
/// (`cargo run`) and `production` for release builds
pub fn mode(obot_env: Option<String>) -> Result<String, Error> {
    let mode = obot_env.filter(|mode| !mode.is_empty()).unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        }
        .to_owned()
    });
    // The mode ends up in a file name
    let valid = mode
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(mode)
    } else {
        Err(format!("`OBOT_ENV` can only hold letters, digits, `-` and `_`, got `{mode}`").into())
    }
}

/// Files read for a mode, most specific first
pub fn files(mode: &str) -> [String; 4] {
    [
        format!(".env.{mode}.local"),
        format!(".env.{mode}"),
        ".env.local".to_owned(),
        ".env".to_owned(),
    ]
}

/// Loads the files of `mode` that exist in the working directory and returns their names. A
/// variable already set, by the real environment or a more specific file, is never overwritten,
/// so each variable gets its most specific value and the real environment always wins.
pub fn load(mode: &str) -> Result<Vec<String>, Error> {
    load_from(Path::new("."), mode)
}

fn load_from(directory: &Path, mode: &str) -> Result<Vec<String>, Error> {
    let mut loaded = Vec::new();
    for file in files(mode) {
        // Only this directory: `dotenvy::from_filename` would also search parent directories
        match dotenvy::from_path(directory.join(&file)) {
            Ok(_) => loaded.push(file),
            Err(error) if error.not_found() => {}
            Err(error) => return Err(format!("Could not read `{file}`: {error}").into()),
        }
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests;
