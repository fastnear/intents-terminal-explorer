# Plugin System Integration Example

Here's how to integrate the plugin system into your NEAR blockchain dashboard:

## 1. Update Cargo.toml

Add the plugin dependencies to your main dashboard:

```toml
[dependencies]
# ... existing dependencies ...
ratacat-plugin-core = { path = "./ratacat-plugin-core" }

# Optional: include plugins as workspace members
[workspace]
members = [
    ".",
    "ratacat-plugin-core",
    "plugins/validator-monitor",
    "plugins/tx-analyzer",
]
```

## 2. Update main.rs

Add plugin initialization to your main function:

```rust
mod plugin_host;
mod plugin_ui;

use plugin_host::DashboardPluginHost;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load();

    // ... existing terminal setup ...

    // Initialize plugin system
    let plugin_host = Arc::new(Mutex::new(DashboardPluginHost::new(tx.clone())));
    plugin_host.lock().await.init().await?;

    // ... rest of main ...
}
```

## 3. Update app.rs

Add plugin integration to your App struct:

```rust
use ratacat_plugin_core::traits::PluginWidget;

pub struct App {
    // ... existing fields ...

    // Plugin state
    plugin_widgets: Vec<PluginWidget>,
    plugin_notifications: Vec<(String, String, NotificationLevel)>,
    plugin_stats: PluginStats,
}

#[derive(Default)]
struct PluginStats {
    active_plugins: usize,
    total_alerts: usize,
    patterns_detected: usize,
}

impl App {
    pub fn on_event(&mut self, ev: AppEvent, keep: usize) {
        match ev {
            // ... existing event handling ...

            AppEvent::PluginNotification { plugin_id, message } => {
                self.plugin_notifications.push((
                    plugin_id,
                    message,
                    NotificationLevel::Info,
                ));

                // Keep only last 5 notifications
                if self.plugin_notifications.len() > 5 {
                    self.plugin_notifications.remove(0);
                }
            }

            AppEvent::PluginMessage(msg) => {
                self.handle_plugin_message(msg);
            }
        }
    }

    fn handle_plugin_message(&mut self, msg: PluginMessage) {
        match msg {
            PluginMessage::ValidatorAlert { .. } => {
                self.plugin_stats.total_alerts += 1;
            }
            PluginMessage::TransactionPattern { .. } => {
                self.plugin_stats.patterns_detected += 1;
            }
            _ => {}
        }
    }
}
```

## 4. Update ui.rs

Add plugin UI rendering:

```rust
use crate::plugin_ui::{
    render_plugin_widgets, create_plugin_status_widget,
    create_validator_health_widget, render_plugin_notifications,
};

pub fn draw(f: &mut Frame, app: &App) {
    // Create main layout with plugin panel
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(100),      // Main content
            Constraint::Length(30),    // Plugin sidebar
        ])
        .split(f.size());

    // Draw main dashboard in left area
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1)
        ])
        .split(main_layout[0]);

    header(f, outer[0], app);
    body(f, outer[1], app);
    footer(f, outer[2], app);

    // Draw plugin panel in right area
    draw_plugin_panel(f, main_layout[1], app);

    // Draw plugin notifications as overlay
    render_plugin_notifications(f, &app.plugin_notifications);
}

fn draw_plugin_panel(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),     // Plugin status
            Constraint::Min(10),       // Validator health
            Constraint::Min(10),       // Plugin widgets
        ])
        .split(area);

    // Plugin status summary
    let status = create_plugin_status_widget(
        app.plugin_stats.active_plugins,
        app.plugin_stats.total_alerts,
        app.plugin_stats.patterns_detected,
    );
    f.render_widget(status, chunks[0]);

    // Validator health (example data)
    let validators = vec![
        ("validator1.near".to_string(), 99.5, true),
        ("validator2.near".to_string(), 85.2, false),
        ("validator3.near".to_string(), 98.7, true),
    ];
    let health = create_validator_health_widget(&validators);
    f.render_widget(health, chunks[1]);

    // Additional plugin widgets
    render_plugin_widgets(f, chunks[2], &app.plugin_widgets);
}
```

## 5. Connect Data Sources to Plugins

Update your data sources to notify plugins:

```rust
// In source_rpc.rs or source_ws.rs
pub async fn run_rpc(cfg: &Config, tx: UnboundedSender<AppEvent>, plugin_host: Arc<Mutex<DashboardPluginHost>>) -> Result<()> {
    // ... existing code ...

    // When receiving a new block
    let row = BlockRow { height: h, tx_count: txs.len(), when, transactions: txs };

    // Notify plugins
    plugin_host.lock().await.notify_block(
        h,
        validator_name,
        txs.len()
    ).await?;

    // Send to UI
    let _ = tx.send(AppEvent::NewBlock(row));
}
```

## 6. Example Plugin Loading

Create a plugin loader in your app:

```rust
// In plugin_host.rs
impl DashboardPluginHost {
    async fn load_builtin_plugins(&self) -> Result<()> {
        let mut registry = self.registry.write().await;

        // Load validator monitor
        use validator_monitor::ValidatorMonitorFactory;
        let validator_factory = Box::new(ValidatorMonitorFactory::new(self.host_impl.clone()));
        registry.register_factory(validator_factory).await?;
        registry.enable_plugin("validator-monitor").await?;

        // Load transaction analyzer
        use tx_analyzer::TransactionAnalyzerFactory;
        let tx_factory = Box::new(TransactionAnalyzerFactory::new(self.host_impl.clone()));
        registry.register_factory(tx_factory).await?;
        registry.enable_plugin("tx-analyzer").await?;

        log::info!("Loaded {} plugins", 2);
        Ok(())
    }
}
```

## 7. Hot Key Bindings for Plugins

Add plugin control keys:

```rust
// In main.rs handle_key function
match (k.code, k.modifiers) {
    // ... existing keys ...

    (KeyCode::Char('p'), _) => app.toggle_plugin_panel(),
    (KeyCode::Char('P'), _) => app.open_plugin_manager(),
    (KeyCode::Char('n'), KeyModifiers::CONTROL) => app.clear_notifications(),
    _ => {}
}
```

## Complete Example Flow

1. **Dashboard starts** → Plugin host initializes
2. **Plugins load** → Register with host, subscribe to events
3. **Block arrives** → Dashboard notifies plugins
4. **Plugin analyzes** → Detects patterns, generates alerts
5. **UI updates** → Shows alerts, stats, and widgets
6. **User interacts** → Can view plugin details, dismiss alerts

This architecture allows plugins to:
- React to blockchain events in real-time
- Provide custom UI components
- Store and retrieve data
- Communicate with other plugins
- Run background analysis

The plugin system is now fully integrated with your NEAR blockchain explorer!