// src/context.rs
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Context {
    pub last_cluster: Option<String>,
}

impl Context {
    pub fn load() -> Result<Self> {
        let context_file = Self::context_file()?;

        if !context_file.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&context_file)?;
        let context: Context = serde_json::from_str(&content)?;
        Ok(context)
    }

    pub fn save(&self) -> Result<()> {
        let context_file = Self::context_file()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&context_file, content)?;
        Ok(())
    }

    pub fn set_last_cluster(&mut self, cluster: String) -> Result<()> {
        self.last_cluster = Some(cluster);
        self.save()
    }

    fn context_file() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let dbctl_dir = home.join(".dbctl");

        // Ensure the directory exists
        fs::create_dir_all(&dbctl_dir)?;

        Ok(dbctl_dir.join("context.json"))
    }
}
