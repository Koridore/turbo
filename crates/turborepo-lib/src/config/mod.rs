mod env;
mod repo;
mod user;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
#[cfg(not(windows))]
use dirs_next::config_dir;
// Go's xdg implementation uses FOLDERID_LocalAppData for config home
// https://github.com/adrg/xdg/blob/master/paths_windows.go#L28
// Rust xdg implementations uses FOLDERID_RoamingAppData for config home
// We use cache_dir so we can find the config dir that the Go code uses
#[cfg(windows)]
use dirs_next::data_local_dir as config_dir;
pub use env::MappedEnvironment;
pub use repo::{RepoConfig, RepoConfigLoader};
use serde::Serialize;
pub use user::{UserConfig, UserConfigLoader};

pub fn default_user_config_path() -> Result<PathBuf> {
    config_dir()
        .map(|p| p.join("turborepo").join("config.json"))
        .context("default config path not found")
}

#[allow(dead_code)]
pub fn data_dir() -> Option<PathBuf> {
    dirs_next::data_dir().map(|p| p.join("turborepo"))
}

fn write_to_disk<T>(path: &Path, config: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent_dir) = path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }
    let config_file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(&config_file, &config)?;
    config_file.sync_all()?;
    Ok(())
}
