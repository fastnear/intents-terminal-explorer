pub mod ipc;
pub mod registry;
pub mod traits;
pub mod types;

pub use ipc::{IPCClient, IPCConnection, IPCServer};
pub use registry::{PluginRegistry, RegistryHost};
pub use traits::*;
pub use types::*;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::traits::{LogLevel, Plugin, PluginFactory, PluginHost};
    pub use crate::types::{
        Capability, PluginConfig, PluginInfo, PluginMessage, QueryType, SubscriptionTopic,
    };
    pub use anyhow::Result;
    pub use async_trait::async_trait;
}
