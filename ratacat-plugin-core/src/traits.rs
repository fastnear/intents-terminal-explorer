use async_trait::async_trait;
use anyhow::Result;
use crate::types::{PluginMessage, PluginInfo, Capability, SubscriptionTopic};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn info(&self) -> PluginInfo;

    /// Initialize the plugin
    async fn init(&mut self) -> Result<()>;

    /// Handle incoming messages
    async fn handle_message(&mut self, message: PluginMessage) -> Result<Option<PluginMessage>>;

    /// Cleanup when plugin is being unloaded
    async fn cleanup(&mut self) -> Result<()>;

    /// Get current subscriptions
    fn subscriptions(&self) -> Vec<SubscriptionTopic> {
        vec![]
    }

    /// Called periodically for housekeeping
    async fn tick(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Plugin host trait - implemented by the main application
#[async_trait]
pub trait PluginHost: Send + Sync {
    /// Send a message to another plugin or the host
    async fn send_message(&self, message: PluginMessage) -> Result<()>;

    /// Query the host for data
    async fn query(&self, message: PluginMessage) -> Result<PluginMessage>;

    /// Subscribe to a topic
    async fn subscribe(&self, topic: SubscriptionTopic) -> Result<()>;

    /// Unsubscribe from a topic
    async fn unsubscribe(&self, topic: SubscriptionTopic) -> Result<()>;

    /// Log a message through the host's logging system
    fn log(&self, level: LogLevel, message: &str);

    /// Get configuration value
    fn get_config(&self, key: &str) -> Option<String>;

    /// Store persistent data
    async fn store_data(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Retrieve persistent data
    async fn get_data(&self, key: &str) -> Result<Option<Vec<u8>>>;
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Factory trait for creating plugins
pub trait PluginFactory: Send + Sync {
    /// Create a new instance of the plugin
    fn create(&self, host: Arc<dyn PluginHost>) -> Result<Box<dyn Plugin>>;

    /// Get plugin info without creating an instance
    fn info(&self) -> PluginInfo;
}

/// Plugin lifecycle hooks
#[async_trait]
pub trait PluginLifecycle {
    /// Called when plugin is first loaded
    async fn on_load(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called when plugin is enabled
    async fn on_enable(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called when plugin is disabled
    async fn on_disable(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called before plugin is unloaded
    async fn on_unload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// UI extension trait for plugins that provide UI components
pub trait PluginUI {
    /// Get custom UI widget for the plugin
    fn get_widget(&self) -> Option<PluginWidget> {
        None
    }

    /// Handle UI events
    fn handle_ui_event(&mut self, event: UIEvent) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PluginWidget {
    StatusBar(String),
    Sidebar {
        title: String,
        content: Vec<String>,
    },
    Modal {
        title: String,
        content: String,
        actions: Vec<String>,
    },
    Notification {
        message: String,
        level: NotificationLevel,
    },
}

#[derive(Debug, Clone)]
pub enum UIEvent {
    KeyPress(char),
    Action(String),
    Focus,
    Blur,
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}