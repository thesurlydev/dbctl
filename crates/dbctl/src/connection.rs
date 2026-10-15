// Connection management for remote PostgreSQL servers

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_postgres::Client;

/// SSL mode for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SslMode {
    #[default]
    Disable,
    Require,
}

impl SslMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SslMode::Disable => "disable",
            SslMode::Require => "require",
        }
    }
}

/// A saved connection profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub ssl_mode: SslMode,
}

impl ConnectionProfile {
    /// Build a connection string for tokio-postgres
    pub fn connection_string(&self, database: Option<&str>) -> String {
        let db = database.unwrap_or(&self.database);
        format!(
            "host={} port={} user={} password='{}' dbname={} sslmode={}",
            self.host,
            self.port,
            self.username,
            self.password,
            db,
            self.ssl_mode.as_str()
        )
    }
}

/// Storage for connection profiles
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionStore {
    pub connections: Vec<ConnectionProfile>,
}

impl ConnectionStore {
    /// Get the path to the connections file
    fn store_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".dbctl")
            .join("connections.json")
    }

    /// Load connections from disk
    pub fn load() -> Result<Self> {
        let path = Self::store_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save connections to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::store_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Add a new connection profile
    pub fn add(&mut self, profile: ConnectionProfile) -> Result<()> {
        if self.connections.iter().any(|c| c.name == profile.name) {
            bail!("Connection '{}' already exists", profile.name);
        }
        self.connections.push(profile);
        self.save()
    }

    /// Remove a connection by name
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let before = self.connections.len();
        self.connections.retain(|c| c.name != name);
        if self.connections.len() == before {
            bail!("Connection '{}' not found", name);
        }
        self.save()
    }

    /// Get a connection by name
    pub fn get(&self, name: &str) -> Option<&ConnectionProfile> {
        self.connections.iter().find(|c| c.name == name)
    }

    /// Update an existing connection
    pub fn update(&mut self, name: &str, profile: ConnectionProfile) -> Result<()> {
        if let Some(c) = self.connections.iter_mut().find(|c| c.name == name) {
            *c = profile;
            self.save()
        } else {
            bail!("Connection '{}' not found", name)
        }
    }
}

/// Unified connection source for both local clusters and remote connections
#[derive(Clone)]
pub enum ConnectionSource {
    /// Local cluster managed by dbctl
    Cluster {
        name: String,
        metadata: crate::server::ClusterMetadata,
    },
    /// Remote PostgreSQL connection
    Remote(ConnectionProfile),
}

impl ConnectionSource {
    /// Create a connection source from a cluster name
    pub fn from_cluster(name: &str) -> Result<Self> {
        let metadata = crate::server::load_cluster_metadata(name)?;
        Ok(ConnectionSource::Cluster {
            name: name.to_string(),
            metadata,
        })
    }

    /// Build connection string for tokio-postgres
    pub fn connection_string(&self, database: Option<&str>) -> String {
        match self {
            ConnectionSource::Cluster { metadata, .. } => {
                metadata.connection_string(database.unwrap_or("postgres"))
            }
            ConnectionSource::Remote(profile) => profile.connection_string(database),
        }
    }

    /// Get the PostgreSQL binaries directory (only for clusters)
    pub fn binaries_dir(&self) -> Option<PathBuf> {
        match self {
            ConnectionSource::Cluster { metadata, .. } => {
                Some(crate::server::get_version_dir(&metadata.version))
            }
            ConnectionSource::Remote(_) => None,
        }
    }

    /// Get username for this connection
    pub fn username(&self) -> &str {
        match self {
            ConnectionSource::Cluster { metadata, .. } => &metadata.username,
            ConnectionSource::Remote(profile) => &profile.username,
        }
    }

    /// Get password (if any)
    pub fn password(&self) -> Option<&str> {
        match self {
            ConnectionSource::Cluster { .. } => None,
            ConnectionSource::Remote(profile) => Some(&profile.password),
        }
    }

    /// Where connections go: a local cluster's socket directory, or a remote
    /// profile's host.
    pub fn host(&self) -> String {
        match self {
            ConnectionSource::Cluster { metadata, .. } => {
                metadata.socket_dir().to_string_lossy().into_owned()
            }
            ConnectionSource::Remote(profile) => profile.host.clone(),
        }
    }

    /// Get port
    pub fn port(&self) -> u16 {
        match self {
            ConnectionSource::Cluster { metadata, .. } => metadata.port,
            ConnectionSource::Remote(profile) => profile.port,
        }
    }

    /// Check if TLS is required
    pub fn requires_tls(&self) -> bool {
        match self {
            ConnectionSource::Cluster { .. } => false,
            ConnectionSource::Remote(profile) => matches!(profile.ssl_mode, SslMode::Require),
        }
    }

    /// Get a display name for this connection
    pub fn display_name(&self) -> &str {
        match self {
            ConnectionSource::Cluster { name, .. } => name,
            ConnectionSource::Remote(profile) => &profile.name,
        }
    }
}

/// Connect to PostgreSQL using the connection source
pub async fn connect(
    source: &ConnectionSource,
    database: Option<&str>,
) -> Result<(Client, tokio::task::JoinHandle<()>)> {
    let conn_str = source.connection_string(database);

    if source.requires_tls() {
        // Create TLS connector using rustls
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(config);
        let (client, connection) = tokio_postgres::connect(&conn_str, tls).await?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok((client, handle))
    } else {
        let (client, connection) =
            tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok((client, handle))
    }
}

/// Test connection to validate credentials and get server version
pub async fn test_connection(source: &ConnectionSource) -> Result<String> {
    let (client, _handle) = connect(source, None).await?;
    let row = client.query_one("SELECT version()", &[]).await?;
    let version: String = row.get(0);
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_string_no_tls() {
        let profile = ConnectionProfile {
            name: "test".to_string(),
            host: "db.example.com".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: SslMode::Disable,
        };

        let conn_str = profile.connection_string(None);
        assert!(conn_str.contains("host=db.example.com"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("sslmode=disable"));
        assert!(conn_str.contains("dbname=mydb"));
    }

    #[test]
    fn test_connection_string_with_tls() {
        let profile = ConnectionProfile {
            name: "test".to_string(),
            host: "db.example.com".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: SslMode::Require,
        };

        let conn_str = profile.connection_string(None);
        assert!(conn_str.contains("sslmode=require"));
    }

    #[test]
    fn test_connection_string_override_database() {
        let profile = ConnectionProfile {
            name: "test".to_string(),
            host: "db.example.com".to_string(),
            port: 5432,
            database: "default_db".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            ssl_mode: SslMode::Disable,
        };

        let conn_str = profile.connection_string(Some("other_db"));
        assert!(conn_str.contains("dbname=other_db"));
        assert!(!conn_str.contains("dbname=default_db"));
    }
}
