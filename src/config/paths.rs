//! XDG path resolution for kaze configuration and data directories.

use anyhow::Result;
use std::path::PathBuf;

use super::types::Config;

impl Config {
    /// Returns the platform-specific configuration directory for kaze.
    ///
    /// Returns `~/.config/kaze/` on Linux (`XDG_CONFIG_HOME/kaze`).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform's config directory cannot be determined.
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join(crate::constants::APP_NAME);
        Ok(dir)
    }

    /// Returns the platform-specific data directory for kaze.
    ///
    /// Returns `~/.local/share/kaze/` on Linux (`XDG_DATA_HOME/kaze`).
    /// Used for storing session history and other persistent data.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform's data directory cannot be determined.
    pub fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
            .join(crate::constants::APP_NAME);
        Ok(dir)
    }

    /// Returns the platform-specific cache directory for kaze.
    ///
    /// Returns `~/.cache/kaze/` on Linux (`XDG_CACHE_HOME/kaze`).
    /// Used for storing readline history and other ephemeral data.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform's cache directory cannot be determined.
    pub fn cache_dir() -> Result<PathBuf> {
        let dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
            .join(crate::constants::APP_NAME);
        Ok(dir)
    }

    /// Returns the full path to the kaze configuration file.
    ///
    /// Returns `~/.config/kaze/config.toml` on Linux.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Config::config_dir`] fails.
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(crate::constants::CONFIG_FILENAME))
    }
}
