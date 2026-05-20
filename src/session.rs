use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "aoc-cli").context("could not determine config directory")
}

pub fn path() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("session"))
}

pub fn save(token: &str) -> Result<PathBuf> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, token).with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("chmod {}", path.display()))?;
    Ok(path)
}

pub fn load() -> Result<String> {
    let path = path()?;
    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "no session found at {} — run `aoc login` first",
            path.display()
        )
    })?;
    Ok(raw.trim().to_string())
}
