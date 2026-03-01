//! Configuration types and path resolution for kaze.
//!
//! Kaze stores its settings as TOML at the platform's XDG config path
//! (e.g. `~/.config/kaze/config.toml` on Linux) and session data under the
//! XDG data directory (`~/.local/share/kaze/`).

mod loader;
mod paths;
mod resolve;
mod types;

#[allow(unused_imports)]
pub use types::CompactionConfig;
pub use types::Config;
#[allow(unused_imports)]
pub use types::ProviderConfig;
#[allow(unused_imports)]
pub use types::ProviderEntry;

use anyhow::Result;

impl Config {
    /// Load config with precedence: project > global > defaults.
    /// Creates default config file if none exists.
    pub fn load() -> Result<Self> {
        let global = Self::load_global()?;
        let project = Self::load_project()?;

        let mut config = global;
        if let Some(proj) = project {
            config = Self::merge(config, proj);
        }

        config.resolve_substitutions();
        Ok(config)
    }
}
