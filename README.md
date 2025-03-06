# Kimiha 🚀

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A high-performance, low-latency event-driven orchestration framework designed to support scalable trading systems with excellent throughput characteristics.

## Overview 📈

Kimiha was specifically developed to address the needs of modern trading systems that require:

- **Microsecond-level responsiveness** for market events
- **High throughput** for processing large volumes of market data
- **Scalability** to handle increased load without performance degradation
- **Reliability** with proper error handling and recovery mechanisms

By leveraging an event-driven architecture at its core, Kimiha enables the construction of trading systems that can efficiently process market data, generate trading signals, and execute orders with minimal latency.

## Features ✨

### Core Orchestration Features

- 🚦 **Dual execution modes**: Background tasks for continuous processing and Event-driven tasks for reactive operations
- 📡 **Configurable event channels** with capacity control for optimal resource management
- ⚡ **Asynchronous execution** powered by Tokio for maximum performance
- 🔒 **Type-safe event system** with compile-time guarantees
- 🔄 **Task lifecycle management** with initialize, execute, and shutdown phases
- 🧩 **Extensible architecture** with trait-based design for easy customization
- 🛡️ **Comprehensive error handling** with custom error types
- 📊 **Task registry** for monitoring and management

### Trading Engine Components

- 🧩 **Pluggable architecture** for modular, maintainable systems
- 🔄 **Component-based design**:
  - **Connectors**: Data collection from exchanges/APIs
  - **Aggregator**: Data normalization and consolidation
  - **Strategies**: Market analysis and signal generation
  - **Executors**: Trade execution and order management

## Installation 📦

Add this to your `Cargo.toml`:

```toml
[dependencies]
kimiha = "0.1.0"
tokio = { version = "1.43.0", features = ["full"] }
async-trait = "0.1.85"
```

## Quick Start 🚀

### Using as a Task Orchestrator

```rust
use kimiha::event_bus::EventBus;
use kimiha::scheduler::Scheduler;
use kimiha::channel_config::ChannelConfig;
use kimiha::types::{EventType, Executable, BackgroundTask};
use kimiha::error::OrchestratorError;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum MyEvent {
    ProcessData,
    ShutdownSignal,
}

impl std::fmt::Display for MyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyEvent::ProcessData => write!(f, "ProcessData"),
            MyEvent::ShutdownSignal => write!(f, "ShutdownSignal"),
        }
    }
}

impl EventType for MyEvent {}

struct BackgroundProcessor {
    name: String,
}

#[async_trait]
impl Executable for BackgroundProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        println!("Initializing {}", self.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        println!("Shutting down {}", self.name);
        Ok(())
    }
}

#[async_trait]
impl BackgroundTask for BackgroundProcessor {
    async fn execute(&mut self) -> Result<(), OrchestratorError> {
        // Your background processing logic here
        println!("Executing background task");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Create event bus with configuration
    let event_bus = EventBus::new(vec![
        (
            MyEvent::ProcessData,
            ChannelConfig {
                capacity: 100,
                description: "Data processing channel".to_string(),
            },
        ),
    ]);
    
    let mut scheduler = Scheduler::new(event_bus);
    
    // Register background task
    let bg_task = BackgroundProcessor {
        name: "DataProcessor".to_string(),
    };
    
    scheduler.register_background_task(Arc::new(Mutex::new(bg_task)));
    
    // Start the scheduler
    scheduler.start().await.unwrap();
    
    // Shutdown after some time
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    scheduler.shutdown().await.unwrap();
}
```

### Building a Trading Engine

```rust
use kimiha::orchestrator::{TradingOrchestratorBuilder};
use kimiha::event_bus::EventBus;
use kimiha::channel_config::ChannelConfig;
use kimiha::types::{EventType, Executable, Connector, Aggregator, Strategy, Executor, EventTask, BackgroundTask};
use kimiha::error::OrchestratorError;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum TradingEvent {
    MarketDataUpdate,
    TradeSignal,
    Shutdown,
}

impl std::fmt::Display for TradingEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradingEvent::MarketDataUpdate => write!(f, "MarketDataUpdate"),
            TradingEvent::TradeSignal => write!(f, "TradeSignal"),
            TradingEvent::Shutdown => write!(f, "Shutdown"),
        }
    }
}

impl EventType for TradingEvent {}

// Example connector component
struct MyConnector {
    name: String,
}

#[async_trait]
impl Executable for MyConnector {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        println!("Initializing connector: {}", self.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        println!("Shutting down connector: {}", self.name);
        Ok(())
    }
}

#[async_trait]
impl BackgroundTask for MyConnector {
    async fn execute(&mut self) -> Result<(), OrchestratorError> {
        println!("Collecting market data");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
}

#[async_trait]
impl Connector for MyConnector {}

// Example strategy component
struct MyStrategy {
    name: String,
    event: TradingEvent,
}

#[async_trait]
impl Executable for MyStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        println!("Initializing strategy: {}", self.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        println!("Shutting down strategy: {}", self.name);
        Ok(())
    }
}

#[async_trait]
impl EventTask<TradingEvent> for MyStrategy {
    fn subscribed_event(&self) -> &TradingEvent {
        &self.event
    }
    
    async fn handle_event(&mut self, event_data: String) -> Result<(), OrchestratorError> {
        println!("Strategy processing event: {}", event_data);
        Ok(())
    }
}

#[async_trait]
impl Strategy<TradingEvent> for MyStrategy {}

// Implement other components similarly...

#[tokio::main]
async fn main() -> Result<(), OrchestratorError> {
    // Create event bus with your custom events
    let event_bus = EventBus::new(vec![
        (
            TradingEvent::MarketDataUpdate,
            ChannelConfig {
                capacity: 1000,
                description: "Market data channel".to_string(),
            },
        ),
        (
            TradingEvent::TradeSignal,
            ChannelConfig {
                capacity: 100,
                description: "Trade signals channel".to_string(),
            },
        ),
    ]);

    // Build and start the engine
    let connector = Arc::new(Mutex::new(MyConnector {
        name: "Binance Connector".to_string(),
    }));
    
    let aggregator = Arc::new(Mutex::new(/* Implement aggregator */));
    let strategy = Arc::new(Mutex::new(MyStrategy {
        name: "Moving Average Strategy".to_string(),
        event: TradingEvent::MarketDataUpdate,
    }));
    let executor = Arc::new(Mutex::new(/* Implement executor */));

    let mut engine = TradingOrchestratorBuilder::new(event_bus)
        .with_connector(connector)
        .with_aggregator(aggregator)
        .with_strategy(strategy)
        .with_executor(executor)
        .build()?;

    engine.start().await?;
    
    // Run until shutdown
    tokio::signal::ctrl_c().await?;
    engine.shutdown().await?;
    
    Ok(())
}
```

## Architecture 🏗️

The framework is built around a modular, event-driven architecture designed for maximum performance:

### Event-driven Core

- **Non-blocking I/O**: Leveraging Tokio for asynchronous operations
- **Broadcast channels**: Efficient multi-consumer message delivery
- **Task isolation**: Independent components that can scale horizontally

### Trading System Layers

1. **Scheduler Core**: Foundational orchestration layer
   - Task registration and lifecycle management
   - Event bus implementation with configurable channels
   - Background and event-driven task execution

2. **Trading Orchestrator**: Higher-level abstraction for trading systems
   - Component-based architecture
   - Coordinated data flow between components
   - Pluggable design for custom implementations

### Data Flow 📊

The standard trading system data flow pattern:

```
Connector(s) → Aggregator → Strategy(s) → Executor(s)
```

Each component communicates through typed events, ensuring type safety and clear separation of concerns.

## Performance Considerations 🚄

Kimiha is specifically designed for trading systems where performance is critical:

- **Memory efficiency**: Minimal allocations in hot paths
- **Lock-free architecture**: Where possible, to avoid contention
- **Backpressure handling**: Through configurable channel capacities
- **Resource management**: Controlled task scheduling and execution

## Use Cases 💼

Kimiha is particularly well-suited for:

- **Algorithmic trading systems**
- **Market making**
- **High-frequency trading applications**
- **Risk management systems**
- **Real-time market data processors**

## License 📄

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.