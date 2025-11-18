use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Core message types for plugin communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginMessage {
    // Blockchain Dashboard Messages
    TransactionFailed {
        hash: String,
        block_height: u64,
        error: String,
        timestamp: DateTime<Utc>,
    },
    BlockProduced {
        height: u64,
        validator: String,
        tx_count: usize,
        timestamp: DateTime<Utc>,
    },
    InterestingTransaction {
        hash: String,
        reason: String,
        signer: String,
        receiver: String,
        actions: Vec<String>,
    },
    ValidatorAlert {
        validator: String,
        alert_type: AlertType,
        message: String,
    },

    // Query Messages
    Query {
        id: Uuid,
        query: QueryType,
    },
    Response {
        id: Uuid,
        data: Value,
        success: bool,
        error: Option<String>,
    },

    // Subscription Messages
    Subscribe {
        topic: SubscriptionTopic,
        subscriber_id: Uuid,
    },
    Unsubscribe {
        topic: SubscriptionTopic,
        subscriber_id: Uuid,
    },

    // Control Messages
    PluginReady {
        plugin_id: String,
        capabilities: Vec<Capability>,
    },
    Ping {
        timestamp: DateTime<Utc>,
    },
    Pong {
        timestamp: DateTime<Utc>,
    },
    Error {
        message: String,
        code: ErrorCode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    GetTodoById(Uuid),
    GetTodosByProject(String),
    GetTodosByTag(String),
    GetBlockByHeight(u64),
    GetTransactionByHash(String),
    GetRecentTransactions { limit: usize },
    GetValidatorStats(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SubscriptionTopic {
    AllTodos,
    TodosInProject(String),
    TodosWithTag(String),
    AllBlocks,
    BlocksFromValidator(String),
    TransactionErrors,
    HighValueTransactions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    TodoManagement,
    BlockchainMonitoring,
    TransactionAnalysis,
    ValidatorTracking,
    CustomQueries,
    RealtimeUpdates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    MissedBlocks,
    LowUptime,
    HighLatency,
    ConfigChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    Unknown = 0,
    InvalidMessage = 1,
    Unauthorized = 2,
    NotFound = 3,
    RateLimited = 4,
    InternalError = 5,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

/// Configuration for plugin connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub socket_path: Option<String>,
    pub tcp_addr: Option<String>,
    pub max_message_size: usize,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
}
