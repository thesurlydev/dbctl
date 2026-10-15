// src/context.rs
//! The last-used cluster, so commands can omit `--cluster`.
//!
//! It is a convenience, never a source of truth, and several dbctl processes
//! can touch it at once (an automation tool may run one per request). So it is
//! written atomically, an unreadable file is ignored rather than fatal, and
//! callers treat a failed save as a warning.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Makes each save's temporary file name unique within the process too.
static SAVES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Context {
    pub last_cluster: Option<String>,
}

impl Context {
    pub fn load() -> Result<Self> {
        Ok(Self::load_from(&Self::context_file()?))
    }

    /// The context stored at `path`; empty when the file is missing or
    /// unreadable. A damaged hint must not break every command that reads it.
    pub fn load_from(path: &Path) -> Self {
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&content) {
            Ok(context) => context,
            Err(e) => {
                eprintln!("Ignoring unreadable {} ({})", path.display(), e);
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::context_file()?)
    }

    /// Write to a temporary file beside `path`, then rename it into place, so
    /// a concurrent reader sees the old file or the new one, never a mix.
    /// Plain `fs::write` from two processes at once interleaved into invalid
    /// JSON.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        let n = SAVES.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), n));
        fs::write(&tmp, content).with_context(|| format!("Failed to write {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("Failed to replace {}", path.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dbctl-context-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips() {
        let dir = scratch("round-trip");
        let path = dir.join("context.json");
        let context = Context {
            last_cluster: Some("main".to_string()),
        };
        context.save_to(&path).unwrap();
        assert_eq!(Context::load_from(&path), context);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_or_damaged_file_reads_as_empty() {
        let dir = scratch("damaged");
        let path = dir.join("context.json");
        assert_eq!(Context::load_from(&path), Context::default());

        // What two concurrent fs::write calls actually produced.
        fs::write(&path, "{\n  \"last_cluster\": \"it_users\"\n}s\"\n}").unwrap();
        assert_eq!(Context::load_from(&path), Context::default());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_saves_never_leave_invalid_json() {
        let dir = scratch("concurrent");
        let path = dir.join("context.json");

        // Alternate a long and a short value from several threads; with a
        // truncating write, a short write over a long one leaves a tail.
        let writers: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for n in 0..200 {
                        let name = if (i + n) % 2 == 0 {
                            "a-much-longer-cluster-name".to_string()
                        } else {
                            "s".to_string()
                        };
                        let _ = Context {
                            last_cluster: Some(name),
                        }
                        .save_to(&path);
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        if !content.is_empty() {
                            serde_json::from_str::<Context>(&content)
                                .unwrap_or_else(|e| panic!("invalid JSON ({e}): {content:?}"));
                        }
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
