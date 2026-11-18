//! PostgreSQL binary caching system
//!
//! Implements a local cache for PostgreSQL binaries to speed up installations
//! and provide resilience against upstream download failures.
//!
//! Cache location: ~/.dbctl/cache/postgres/. Every archive, cached or
//! freshly downloaded, is checked against the SHA-256 pinned in
//! `server::AVAILABLE_VERSIONS` before it is unpacked.
//!
//! Only macOS downloads PostgreSQL, so in practice only macOS archives are
//! cached. On Linux, `dbctl install` links the distribution's packages
//! (apt.postgresql.org) instead: EnterpriseDB publishes no Linux binaries
//! for current versions.
//!
//! Cache keys: {version}-{os}-{arch}.{ext}, e.g. 17.7-macos-universal.zip
//! ("universal": the binaries run on both Intel and Apple Silicon).

use anyhow::Result;
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

/// The cache directory, ~/.dbctl/cache/postgres. It lives under the user's
/// own home on purpose: a shared directory would let another user plant an
/// archive (every archive is still checked against its pinned hash).
pub fn get_cache_dir() -> Result<PathBuf> {
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

/// Get the path to a cached binary (doesn't check if it exists)
pub fn get_cache_path(version: &str, os: &str, arch: &str) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    let key = cache_key(version, os, arch);
    Ok(cache_dir.join(key))
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
