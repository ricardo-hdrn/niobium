use std::path::PathBuf;

/// Runtime configuration for Niobium.
#[derive(Debug, Clone)]
pub struct Config {
    /// Base directory for Niobium data: ~/.niobium
    pub data_dir: PathBuf,
    /// Timeout (seconds) waiting for a form submission
    pub form_timeout: u64,
}

impl Config {
    pub fn load() -> Self {
        let data_dir = if let Ok(dir) = std::env::var("NIOBIUM_DATA_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::home_dir()
                .expect("cannot determine home directory")
                .join(".niobium")
        };

        Self {
            data_dir,
            form_timeout: 600, // 10 minutes
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("niobium.db")
    }
}
