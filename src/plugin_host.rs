use anyhow::Result;
use async_trait::async_trait;
use ratacat_plugin_core::{
    PluginHost, PluginMessage, PluginRegistry, SubscriptionTopic,
    LogLevel, QueryType, types::*
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::types::AppEvent;

/// Dashboard plugin host implementation
pub struct DashboardPluginHost {
    registry: Arc<RwLock<PluginRegistry>>,
    app_tx: mpsc::UnboundedSender<AppEvent>,
    config_store: Arc<RwLock<std::collections::HashMap<String, String>>>,
    data_store: Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
}

impl DashboardPluginHost {
    pub fn new(app_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        let host_impl = Arc::new(DashboardHostImpl {
            app_tx: app_tx.clone(),
            config_store: Arc::new(RwLock::new(std::collections::HashMap::new())),
            data_store: Arc::new(RwLock::new(std::collections::HashMap::new())),
        });

        let registry = PluginRegistry::new(host_impl.clone() as Arc<dyn PluginHost>);

        Self {
            registry: Arc::new(RwLock::new(registry)),
            app_tx,
            config_store: host_impl.config_store.clone(),
            data_store: host_impl.data_store.clone(),
        }
    }

    /// Initialize the plugin system
    pub async fn init(&mut self) -> Result<()> {
        // Start IPC server for external plugins
        let mut registry = self.registry.write().await;
        registry.start_ipc_server("/tmp/ratacat-plugins.sock").await?;

        // Load built-in plugins
        self.load_builtin_plugins().await?;

        Ok(())
    }

    /// Load built-in plugins
    async fn load_builtin_plugins(&self) -> Result<()> {
        // In a real implementation, these would be dynamically loaded
        // For now, we'll use a placeholder

        log::info!("Loading built-in plugins...");

        // Example: Load validator monitor plugin
        // let validator_factory = Box::new(ValidatorMonitorFactory::new(self.host_impl.clone()));
        // registry.register_factory(validator_factory).await?;

        Ok(())
    }

    /// Handle messages from plugins
    pub async fn handle_plugin_message(&self, message: PluginMessage) -> Result<()> {
        match message {
            PluginMessage::InterestingTransaction { hash, reason, .. } => {
                // Convert to app event and send to UI
                let event = AppEvent::PluginNotification {
                    plugin_id: "tx-analyzer".to_string(),
                    message: format!("Interesting tx {}: {}", hash, reason),
                };
                let _ = self.app_tx.send(event);
            }

            PluginMessage::ValidatorAlert { validator, message, .. } => {
                let event = AppEvent::PluginNotification {
                    plugin_id: "validator-monitor".to_string(),
                    message: format!("Validator {}: {}", validator, message),
                };
                let _ = self.app_tx.send(event);
            }

            _ => {
                // Route to registry for handling
                let registry = self.registry.read().await;
                registry.broadcast(message).await?;
            }
        }
        Ok(())
    }

    /// Send blockchain event to plugins
    pub async fn notify_block(&self, height: u64, validator: String, tx_count: usize) -> Result<()> {
        let message = PluginMessage::BlockProduced {
            height,
            validator,
            tx_count,
            timestamp: chrono::Utc::now(),
        };

        let registry = self.registry.read().await;
        registry.broadcast(message).await
    }

    /// Send transaction event to plugins
    pub async fn notify_transaction(
        &self,
        hash: String,
        signer: String,
        receiver: String,
        actions: Vec<String>,
    ) -> Result<()> {
        let message = PluginMessage::InterestingTransaction {
            hash,
            reason: "New transaction".to_string(),
            signer,
            receiver,
            actions,
        };

        let registry = self.registry.read().await;
        registry.broadcast(message).await
    }

    /// Get plugin UI components for rendering
    pub async fn get_plugin_widgets(&self) -> Vec<PluginWidget> {
        // Collect UI widgets from all enabled plugins
        Vec::new() // Placeholder
    }
}

/// Internal host implementation
struct DashboardHostImpl {
    app_tx: mpsc::UnboundedSender<AppEvent>,
    config_store: Arc<RwLock<std::collections::HashMap<String, String>>>,
    data_store: Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
}

#[async_trait]
impl PluginHost for DashboardHostImpl {
    async fn send_message(&self, message: PluginMessage) -> Result<()> {
        // Convert to app event and send
        match message {
            PluginMessage::Error { message, .. } => {
                log::error!("Plugin error: {}", message);
            }
            _ => {
                // Forward to app for handling
                let event = AppEvent::PluginMessage(message);
                let _ = self.app_tx.send(event);
            }
        }
        Ok(())
    }

    async fn query(&self, message: PluginMessage) -> Result<PluginMessage> {
        // Handle queries - in real implementation would route to appropriate handler
        match message {
            PluginMessage::Query { id, query } => {
                let response = match query {
                    QueryType::GetBlockByHeight(height) => {
                        // Would query blockchain data
                        PluginMessage::Response {
                            id,
                            data: serde_json::json!({
                                "height": height,
                                "placeholder": true
                            }),
                            success: false,
                            error: Some("Not implemented".to_string()),
                        }
                    }
                    _ => PluginMessage::Response {
                        id,
                        data: serde_json::Value::Null,
                        success: false,
                        error: Some("Query type not supported".to_string()),
                    },
                };
                Ok(response)
            }
            _ => Err(anyhow::anyhow!("Invalid query message")),
        }
    }

    async fn subscribe(&self, topic: SubscriptionTopic) -> Result<()> {
        log::info!("Plugin subscribed to {:?}", topic);
        Ok(())
    }

    async fn unsubscribe(&self, topic: SubscriptionTopic) -> Result<()> {
        log::info!("Plugin unsubscribed from {:?}", topic);
        Ok(())
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Trace => log::trace!("[Plugin] {}", message),
            LogLevel::Debug => log::debug!("[Plugin] {}", message),
            LogLevel::Info => log::info!("[Plugin] {}", message),
            LogLevel::Warn => log::warn!("[Plugin] {}", message),
            LogLevel::Error => log::error!("[Plugin] {}", message),
        }
    }

    fn get_config(&self, key: &str) -> Option<String> {
        // Check environment first, then config store
        std::env::var(key).ok()
    }

    async fn store_data(&self, key: &str, value: &[u8]) -> Result<()> {
        let mut store = self.data_store.write().await;
        store.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn get_data(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let store = self.data_store.read().await;
        Ok(store.get(key).cloned())
    }
}

// Add plugin-related events to AppEvent
#[derive(Debug, Clone)]
pub enum AppEvent {
    // ... existing events ...
    PluginMessage(PluginMessage),
    PluginNotification { plugin_id: String, message: String },
}