// Connections to a dbctl-managed cluster, always over its unix socket.

use anyhow::Result;
use tokio_postgres::Client;

/// Where dbctl connects: one of its own clusters, reached through the
/// cluster's unix socket, so it works under either auth mode without a
/// password.
#[derive(Clone)]
pub struct ConnectionSource {
    pub name: String,
    pub metadata: crate::server::ClusterMetadata,
}

impl ConnectionSource {
    /// Create a connection source from a cluster name
    pub fn from_cluster(name: &str) -> Result<Self> {
        let metadata = crate::server::load_cluster_metadata(name)?;
        Ok(ConnectionSource {
            name: name.to_string(),
            metadata,
        })
    }

    /// Build connection string for tokio-postgres
    pub fn connection_string(&self, database: Option<&str>) -> String {
        self.metadata
            .connection_string(database.unwrap_or("postgres"))
    }
}

/// Connect to PostgreSQL using the connection source
pub async fn connect(
    source: &ConnectionSource,
    database: Option<&str>,
) -> Result<(Client, tokio::task::JoinHandle<()>)> {
    let conn_str = source.connection_string(database);
    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    let handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok((client, handle))
}
