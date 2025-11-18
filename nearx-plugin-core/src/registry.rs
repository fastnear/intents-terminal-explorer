use crate::ipc::{IPCConnection, IPCServer};
use crate::traits::{LogLevel, Plugin, PluginFactory, PluginHost};
use crate::types::{Capability, PluginInfo, PluginMessage, SubscriptionTopic};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};

/// Plugin instance with metadata
struct PluginInstance {
    plugin: Box<dyn Plugin>,
    info: PluginInfo,
    enabled: bool,
    subscriptions: Vec<SubscriptionTopic>,
    connection: Option<IPCConnection>,
}

/// Plugin registry that manages all plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    host_impl: Arc<dyn PluginHost>,
    message_bus: Arc<MessageBus>,
    ipc_server: Option<IPCServer>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(host: Arc<dyn PluginHost>) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            host_impl: host,
            message_bus: Arc::new(MessageBus::new()),
            ipc_server: None,
        }
    }

    /// Start IPC server for external plugins
    pub async fn start_ipc_server(&mut self, socket_path: &str) -> Result<()> {
        self.ipc_server = Some(IPCServer::bind_unix(socket_path).await?);

        let plugins = self.plugins.clone();
        let message_bus = self.message_bus.clone();

        // Spawn IPC accept loop
        tokio::spawn(async move {
            if let Some(server) = &self.ipc_server {
                loop {
                    match server.accept().await {
                        Ok(mut conn) => {
                            let plugins = plugins.clone();
                            let bus = message_bus.clone();

                            // Handle connection in separate task
                            tokio::spawn(async move {
                                while let Some(msg) = conn.rx.recv().await {
                                    // Route message through message bus
                                    let _ = bus.publish(msg).await;
                                }
                            });
                        }
                        Err(e) => {
                            log::error!("Failed to accept IPC connection: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Register a plugin factory
    pub async fn register_factory(&self, factory: Box<dyn PluginFactory>) -> Result<()> {
        let info = factory.info();
        let plugin = factory.create(self.host_impl.clone())?;
        self.register_plugin(info.id.clone(), plugin).await
    }

    /// Register a plugin instance
    pub async fn register_plugin(&self, id: String, mut plugin: Box<dyn Plugin>) -> Result<()> {
        // Initialize plugin
        plugin.init().await?;

        let info = plugin.info();
        let subscriptions = plugin.subscriptions();

        // Subscribe to topics
        for topic in &subscriptions {
            self.message_bus
                .subscribe(id.clone(), topic.clone())
                .await?;
        }

        let instance = PluginInstance {
            plugin,
            info: info.clone(),
            enabled: false,
            subscriptions,
            connection: None,
        };

        let mut plugins = self.plugins.write().await;
        plugins.insert(id.clone(), instance);

        // Notify plugin is ready
        let ready_msg = PluginMessage::PluginReady {
            plugin_id: id,
            capabilities: info.capabilities,
        };
        self.message_bus.publish(ready_msg).await?;

        Ok(())
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(instance) = plugins.get_mut(id) {
            instance.enabled = true;
            Ok(())
        } else {
            Err(anyhow!("Plugin {} not found", id))
        }
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(instance) = plugins.get_mut(id) {
            instance.enabled = false;
            Ok(())
        } else {
            Err(anyhow!("Plugin {} not found", id))
        }
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(mut instance) = plugins.remove(id) {
            // Cleanup plugin
            instance.plugin.cleanup().await?;

            // Unsubscribe from topics
            for topic in &instance.subscriptions {
                self.message_bus.unsubscribe(id, topic.clone()).await?;
            }

            Ok(())
        } else {
            Err(anyhow!("Plugin {} not found", id))
        }
    }

    /// Send a message to all plugins
    pub async fn broadcast(&self, message: PluginMessage) -> Result<()> {
        self.message_bus.publish(message).await
    }

    /// Route a message to specific plugin
    pub async fn send_to_plugin(&self, plugin_id: &str, message: PluginMessage) -> Result<()> {
        let plugins = self.plugins.read().await;
        if let Some(instance) = plugins.get(plugin_id) {
            if !instance.enabled {
                return Err(anyhow!("Plugin {} is disabled", plugin_id));
            }

            // Send through IPC if external plugin
            if let Some(conn) = &instance.connection {
                conn.send(message).await?;
            } else {
                // Direct call for in-process plugin
                drop(plugins); // Release read lock
                let mut plugins = self.plugins.write().await;
                if let Some(instance) = plugins.get_mut(plugin_id) {
                    instance.plugin.handle_message(message).await?;
                }
            }
            Ok(())
        } else {
            Err(anyhow!("Plugin {} not found", plugin_id))
        }
    }

    /// Get list of all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|i| i.info.clone()).collect()
    }

    /// Run periodic tick on all enabled plugins
    pub async fn tick_all(&self) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        for (_, instance) in plugins.iter_mut() {
            if instance.enabled {
                instance.plugin.tick().await?;
            }
        }
        Ok(())
    }
}

/// Message bus for routing messages between plugins
struct MessageBus {
    subscriptions: Arc<RwLock<HashMap<SubscriptionTopic, Vec<String>>>>,
    handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<PluginMessage>>>>,
}

impl MessageBus {
    fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn subscribe(&self, plugin_id: String, topic: SubscriptionTopic) -> Result<()> {
        let mut subs = self.subscriptions.write().await;
        subs.entry(topic).or_insert_with(Vec::new).push(plugin_id);
        Ok(())
    }

    async fn unsubscribe(&self, plugin_id: &str, topic: SubscriptionTopic) -> Result<()> {
        let mut subs = self.subscriptions.write().await;
        if let Some(plugins) = subs.get_mut(&topic) {
            plugins.retain(|id| id != plugin_id);
        }
        Ok(())
    }

    async fn publish(&self, message: PluginMessage) -> Result<()> {
        let topic = match &message {
            PluginMessage::TodoCreated { .. } => Some(SubscriptionTopic::AllTodos),
            PluginMessage::TodoCompleted { .. } => Some(SubscriptionTopic::AllTodos),
            PluginMessage::TodoDeleted { .. } => Some(SubscriptionTopic::AllTodos),
            PluginMessage::BlockProduced { .. } => Some(SubscriptionTopic::AllBlocks),
            PluginMessage::TransactionFailed { .. } => Some(SubscriptionTopic::TransactionErrors),
            _ => None,
        };

        if let Some(topic) = topic {
            let subs = self.subscriptions.read().await;
            if let Some(plugin_ids) = subs.get(&topic) {
                let handlers = self.handlers.read().await;
                for plugin_id in plugin_ids {
                    if let Some(tx) = handlers.get(plugin_id) {
                        let _ = tx.send(message.clone());
                    }
                }
            }
        }

        Ok(())
    }

    async fn register_handler(&self, plugin_id: String, tx: mpsc::UnboundedSender<PluginMessage>) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(plugin_id, tx);
    }
}

/// Host implementation for plugin registry
pub struct RegistryHost {
    registry: Arc<RwLock<PluginRegistry>>,
}

#[async_trait]
impl PluginHost for RegistryHost {
    async fn send_message(&self, message: PluginMessage) -> Result<()> {
        let registry = self.registry.read().await;
        registry.broadcast(message).await
    }

    async fn query(&self, message: PluginMessage) -> Result<PluginMessage> {
        // Route query to appropriate plugin based on message type
        // This is a simplified implementation
        match message {
            PluginMessage::Query { id, query: _ } => Ok(PluginMessage::Response {
                id,
                data: serde_json::Value::Null,
                success: false,
                error: Some("Query routing not implemented".to_string()),
            }),
            _ => Err(anyhow!("Invalid query message")),
        }
    }

    async fn subscribe(&self, topic: SubscriptionTopic) -> Result<()> {
        // Subscription handled at registry level
        Ok(())
    }

    async fn unsubscribe(&self, topic: SubscriptionTopic) -> Result<()> {
        // Unsubscription handled at registry level
        Ok(())
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Trace => log::trace!("{}", message),
            LogLevel::Debug => log::debug!("{}", message),
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warn => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
        }
    }

    fn get_config(&self, key: &str) -> Option<String> {
        // Delegate to actual config system
        std::env::var(key).ok()
    }

    async fn store_data(&self, key: &str, value: &[u8]) -> Result<()> {
        // Simplified - in real implementation would use persistent storage
        Ok(())
    }

    async fn get_data(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Simplified - in real implementation would use persistent storage
        Ok(None)
    }
}
