# Ratacat Plugin System Architecture

## Overview

The Ratacat Plugin System enables bidirectional communication between the todo app and blockchain dashboard, allowing them to share data and functionality.

## Architecture Components

### 1. Core Plugin Infrastructure

```
ratacat-plugin-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs      # Plugin trait definitions
│   ├── types.rs       # Shared data types
│   ├── ipc.rs         # IPC communication layer
│   └── registry.rs    # Plugin registry

ratacat (todo app)/
├── src/
│   ├── plugins/
│   │   ├── mod.rs
│   │   └── blockchain.rs
│   └── plugin_host.rs

ratacat-dashboard/
├── src/
│   ├── plugins/
│   │   ├── mod.rs
│   │   └── todos.rs
│   └── plugin_host.rs
```

### 2. Plugin Communication Flow

```
┌─────────────────┐         IPC          ┌──────────────────┐
│   Todo App      │◄─────────────────────►│  NEAR Dashboard  │
│                 │                       │                  │
│ ┌─────────────┐ │    Unix Socket/TCP   │ ┌──────────────┐ │
│ │ Blockchain  │ │                       │ │ Todo Plugin  │ │
│ │   Plugin    │ │   ┌─────────────┐    │ │              │ │
│ └─────────────┘ │   │ Message Bus │    │ └──────────────┘ │
└─────────────────┘   └─────────────┘    └──────────────────┘
```

### 3. Plugin Capabilities

#### Todo App Plugins:
- **Blockchain Plugin**:
  - Create todos from failed transactions
  - Set reminders for block events
  - Track on-chain task completion
  - Monitor validator performance goals

#### Dashboard Plugins:
- **Todo Plugin**:
  - Show blockchain-related todos in sidebar
  - Create todos from transaction details
  - Mark transactions for follow-up
  - Track investigation tasks

### 4. Message Types

```rust
pub enum PluginMessage {
    // From Todo App
    TodoCreated { id: Uuid, title: String, metadata: Value },
    TodoCompleted { id: Uuid },
    TodoDeleted { id: Uuid },

    // From Dashboard
    TransactionFailed { hash: String, error: String },
    BlockProduced { height: u64, validator: String },
    InterestingTransaction { hash: String, reason: String },

    // Bidirectional
    Query { id: Uuid, query: QueryType },
    Response { id: Uuid, data: Value },
    Subscribe { topic: String },
    Unsubscribe { topic: String },
}
```

### 5. Security Considerations

- Plugins run in sandboxed environment
- Messages are validated and sanitized
- Rate limiting on IPC channels
- Optional encryption for sensitive data
- Capability-based permissions

### 6. Plugin Lifecycle

1. **Discovery**: Apps scan plugin directories on startup
2. **Loading**: Plugins are loaded dynamically
3. **Registration**: Plugins register capabilities and subscriptions
4. **Communication**: Bidirectional message passing via IPC
5. **Cleanup**: Graceful shutdown with state persistence

## Implementation Phases

### Phase 1: Core Infrastructure
- Plugin traits and types
- Basic IPC communication
- Simple plugin loader

### Phase 2: Plugin Implementation
- Blockchain plugin for todo app
- Todo plugin for dashboard
- Message routing

### Phase 3: Advanced Features
- Plugin marketplace
- Hot reloading
- Cross-machine communication
- Plugin sandboxing