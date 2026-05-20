use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::io;
use std::path::PathBuf;

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "aoc-cli").context("could not determine cache directory")
}

pub fn input_path(year: u32, day: u32) -> Result<PathBuf> {
    Ok(project_dirs()?
        .cache_dir()
        .join("inputs")
        .join(year.to_string())
        .join(format!("{day:02}.txt")))
}

pub fn read_input(year: u32, day: u32) -> Result<Option<String>> {
    let path = input_path(year, day)?;
    match fs::read_to_string(&path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

pub fn write_input(year: u32, day: u32, body: &str) -> Result<PathBuf> {
    let path = input_path(year, day)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}
