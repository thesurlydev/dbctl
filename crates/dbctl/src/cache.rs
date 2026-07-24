//! PostgreSQL binary caching system
//!
//! Implements a local cache for PostgreSQL binaries to speed up installations
//! and provide resilience against upstream download failures.
//!
//! Cache locations:
//! - Local dev: ~/.dbctl/cache/postgres/
//! - VPS: /data/cache/postgres/
//!
//! Cache keys: {version}-{os}-{arch}.{ext}
//!
//! Architecture handling:
//! - macOS: Uses "universal" (binaries work on both Intel and Apple Silicon)
//! - Linux: Uses "x64" (amd64) or "arm64" (aarch64) - architecture-specific
//!
//! Examples:
//! - 17.7-macos-universal.zip (works on all macOS)
//! - 17.7-linux-x64.tar.gz (amd64 only)
//! - 17.7-linux-arm64.tar.gz (aarch64 only)

use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum CacheAction {
    /// Show cache statistics
    Stats,

    /// Clear all cached PostgreSQL binaries
    Clear,
}

pub fn handle_action(action: CacheAction) -> Result<()> {
    match action {
        CacheAction::Stats => {
            let stats = cache_stats()?;
            println!("Cache location: {}", stats.location.display());
            println!("Cached files:   {}", stats.total_files);
            println!("Total size:     {} MB", stats.total_size_mb);
        }
        CacheAction::Clear => {
            clear_cache()?;
            println!("✓ Cache cleared");
        }
    }
    Ok(())
}

/// Get the cache directory for PostgreSQL binaries
pub fn get_cache_dir() -> Result<PathBuf> {
    // Try /data/cache/postgres first (VPS)
    let vps_cache = PathBuf::from("/data/cache/postgres");
    if vps_cache.parent().map(|p| p.exists()).unwrap_or(false) {
        fs::create_dir_all(&vps_cache)?;
        return Ok(vps_cache);
    }

    // Fall back to ~/.dbctl/cache/postgres (local dev)
    let home_cache = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".dbctl")
        .join("cache")
        .join("postgres");

    fs::create_dir_all(&home_cache)?;
    Ok(home_cache)
}

/// Generate cache key for a PostgreSQL binary
pub fn cache_key(version: &str, os: &str, arch: &str) -> String {
    let ext = if os == "macos" { "zip" } else { "tar.gz" };
    format!("{}-{}-{}.{}", version, os, arch, ext)
}

/// Check if a binary exists in the cache
pub fn is_cached(version: &str, os: &str, arch: &str) -> Result<bool> {
    let cache_dir = get_cache_dir()?;
    let key = cache_key(version, os, arch);
    let cache_path = cache_dir.join(&key);
    Ok(cache_path.exists())
}

/// Get the path to a cached binary (doesn't check if it exists)
pub fn get_cache_path(version: &str, os: &str, arch: &str) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    let key = cache_key(version, os, arch);
    Ok(cache_dir.join(key))
}

/// Save downloaded binary to cache
pub fn save_to_cache(version: &str, os: &str, arch: &str, data: &[u8]) -> Result<PathBuf> {
    let cache_path = get_cache_path(version, os, arch)?;
    fs::write(&cache_path, data)
        .with_context(|| format!("Failed to write cache file: {}", cache_path.display()))?;
    Ok(cache_path)
}

/// Get cache statistics
pub fn cache_stats() -> Result<CacheStats> {
    let cache_dir = get_cache_dir()?;

    if !cache_dir.exists() {
        return Ok(CacheStats {
            total_files: 0,
            total_size_mb: 0,
            location: cache_dir,
        });
    }

    let mut total_files = 0;
    let mut total_size = 0u64;

    for entry in fs::read_dir(&cache_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            total_files += 1;
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }

    Ok(CacheStats {
        total_files,
        total_size_mb: total_size / (1024 * 1024),
        location: cache_dir,
    })
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_mb: u64,
    pub location: PathBuf,
}

/// Clear all cached binaries
pub fn clear_cache() -> Result<()> {
    let cache_dir = get_cache_dir()?;

    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_linux() {
        let key = cache_key("17.7", "linux", "x64");
        assert_eq!(key, "17.7-linux-x64.tar.gz");
    }

    #[test]
    fn test_cache_key_macos() {
        let key = cache_key("16.11", "macos", "universal");
        assert_eq!(key, "16.11-macos-universal.zip");
    }

    #[test]
    fn test_cache_key_format() {
        // Ensure cache keys are filesystem-safe
        let key = cache_key("17.7", "linux", "x64");
        assert!(!key.contains('/'));
        assert!(!key.contains('\\'));
        assert!(!key.contains(' '));
    }
}
