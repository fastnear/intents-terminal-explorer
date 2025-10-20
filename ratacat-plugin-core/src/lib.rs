pub mod types;
pub mod traits;
pub mod ipc;
pub mod registry;

pub use types::*;
pub use traits::*;
pub use ipc::{IPCClient, IPCServer, IPCConnection};
pub use registry::{PluginRegistry, RegistryHost};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::traits::{Plugin, PluginHost, PluginFactory, LogLevel};
    pub use crate::types::{
        PluginMessage, PluginInfo, PluginConfig, Capability,
        SubscriptionTopic, QueryType
    };
    pub use async_trait::async_trait;
    pub use anyhow::Result;
}