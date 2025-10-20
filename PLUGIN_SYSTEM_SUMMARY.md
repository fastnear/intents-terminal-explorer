# 🚀 NEAR Blockchain Explorer Plugin System

## What We Built

We've created a powerful, extensible plugin system for your NEAR Protocol blockchain explorer that enables:

### Core Infrastructure (`ratacat-plugin-core`)
- **Plugin Traits**: Clean interfaces for plugins to implement
- **IPC Communication**: Unix sockets & TCP for external plugin processes
- **Message Bus**: Event-driven architecture for plugin communication
- **Registry**: Dynamic plugin loading and lifecycle management

### Example Plugins

#### 1. Validator Monitor Plugin
- Tracks validator performance and uptime
- Alerts on missed blocks, low uptime, high latency
- Real-time health monitoring
- Historical stats tracking

#### 2. Transaction Analyzer Plugin
- Pattern detection (MEV, high-value transfers, batch txs)
- Risk scoring for transactions
- Automatic action decoding
- Trend analysis and insights

### UI Integration
- Plugin widgets (sidebars, modals, notifications)
- Real-time status indicators
- Validator health visualization
- Pattern detection gauges
- Overlay notifications for alerts

## Architecture Highlights

```
┌─────────────────────────────────────────────────┐
│           NEAR Blockchain Explorer              │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌─────────────┐      ┌──────────────────┐    │
│  │   Main UI   │◄─────│  Plugin Host     │    │
│  └─────────────┘      └────────┬─────────┘    │
│                                │               │
│  ┌─────────────────────────────┼──────────┐   │
│  │          Plugin Registry     │          │   │
│  └──────────────┬──────────────┴──────────┘   │
│                 │                              │
│     ┌───────────┴────────────┬────────────┐   │
│     ▼                        ▼            ▼   │
│ ┌────────┐            ┌────────┐    ┌────────┐│
│ │Validator│            │   TX   │    │Contract││
│ │Monitor  │            │Analyzer│    │Decoder ││
│ └────────┘            └────────┘    └────────┘│
└─────────────────────────────────────────────────┘
                       │
                       ▼
                 External Plugins
                  (via IPC)
```

## Key Features

### 1. **Hot-Pluggable Architecture**
- Plugins can be loaded/unloaded at runtime
- No dashboard restart required
- Graceful degradation if plugin fails

### 2. **Rich Communication**
```rust
pub enum PluginMessage {
    BlockProduced { height, validator, tx_count, timestamp },
    TransactionFailed { hash, error, timestamp },
    ValidatorAlert { validator, alert_type, message },
    Query { id, query_type },
    Response { id, data, success },
    // ... and more
}
```

### 3. **Flexible UI Extensions**
- StatusBar indicators
- Sidebar panels
- Modal dialogs
- Toast notifications
- Custom widgets

### 4. **Performance Optimized**
- Async message passing
- Non-blocking IPC
- Efficient event routing
- Minimal overhead

## Example Usage

```rust
// Plugin subscribes to blockchain events
fn subscriptions(&self) -> Vec<SubscriptionTopic> {
    vec![
        SubscriptionTopic::AllBlocks,
        SubscriptionTopic::TransactionErrors,
        SubscriptionTopic::HighValueTransactions,
    ]
}

// Plugin receives and analyzes events
async fn handle_message(&mut self, msg: PluginMessage) -> Result<Option<PluginMessage>> {
    match msg {
        PluginMessage::BlockProduced { validator, .. } => {
            self.update_validator_stats(validator).await;
            // Return alert if issues detected
        }
        _ => {}
    }
    Ok(None)
}
```

## Benefits

1. **Extensibility**: Add new analysis capabilities without modifying core
2. **Modularity**: Each plugin focuses on specific functionality
3. **Reusability**: Plugins can be shared between different NEAR tools
4. **Isolation**: Plugin crashes don't affect the main dashboard
5. **Community**: Others can contribute plugins for specific use cases

## Future Plugin Ideas

- **Smart Contract Decoder**: Decode contract calls and state changes
- **DeFi Monitor**: Track DEX activity, liquidity, yields
- **NFT Tracker**: Monitor NFT mints, transfers, and marketplaces
- **Governance Watcher**: Track DAO proposals and votes
- **Network Stats**: Aggregate network-wide statistics
- **Alert Manager**: Configurable alerts with webhooks/email
- **Data Exporter**: Export blockchain data in various formats

## Running with Plugins

```bash
# Start dashboard with plugins
ENABLE_PLUGINS=true cargo run

# Or with specific plugins
PLUGINS=validator-monitor,tx-analyzer cargo run

# External plugin connection
./external-plugin --connect /tmp/ratacat-plugins.sock
```

This plugin system transforms your NEAR explorer from a static viewer into a dynamic, extensible platform for blockchain analysis! 🎉