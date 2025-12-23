use serde::{Deserialize, Serialize};

/// Main application tabs
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Overview,
    Databases,
    Users,
    Logs,
    Sql,
    Tables,
    SchemaDesigner,
}

/// Export format for table data
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ExportFormat {
    #[default]
    Csv,
    Json,
}

/// Results from async database operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AsyncResult {
    Databases(String, Vec<String>),
    Users(String, Vec<dbctl::UserInfo>),
    Stats(String, dbctl::ClusterStats),
    SqlTabResult(usize, String),
    Message(String),
    Error(String),
    // Table browser results
    Schemas(String, String, Vec<dbctl::SchemaInfo>),
    SchemaTables(String, String, String, Vec<dbctl::TableInfo>),
    TableStructure(String, String, String, String, dbctl::TableStructure),
    TableData(String, String, String, String, dbctl::TableDataResult),
    // Remote connection results
    ConnectionTest(String, Result<String, String>),
    RemoteDatabases(String, Vec<String>),
    RemoteUsers(String, Vec<dbctl::UserInfo>),
    RemoteStats(String, dbctl::ClusterStats),
    RemoteSchemas(String, String, Vec<dbctl::SchemaInfo>),
    RemoteTables(String, String, String, Vec<dbctl::TableInfo>),
    RemoteTableStructure(String, String, String, String, dbctl::TableStructure),
    RemoteTableData(String, String, String, String, dbctl::TableDataResult),
    // Schema designer results
    SchemaLoaded(String, String, Vec<dbctl::TableStructure>),
    SchemaLoadError(String),
    MigrationApplied(String),
    MigrationFailed(String),
    // AI Chat results
    AiChatResponse(String, Option<crate::ai_chat::SchemaChangeProposal>),
    AiChatError(String),
}

/// Local cluster information
#[derive(Clone)]
pub struct ClusterInfo {
    pub name: String,
    pub metadata: dbctl::ClusterMetadata,
    pub running: bool,
    pub stats: Option<dbctl::ClusterStats>,
}

/// Remote connection information
#[derive(Clone)]
pub struct ConnectionInfo {
    pub profile: dbctl::ConnectionProfile,
    pub connected: bool,
    pub databases: Vec<String>,
}

/// Tracks which connection source is active
#[derive(Clone, PartialEq)]
pub enum ActiveConnection {
    None,
    Cluster(usize),
    Remote(usize),
}
