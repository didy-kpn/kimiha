use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    error::OrchestratorError,
    event_bus::EventBus,
    scheduler::Scheduler,
    task_id::TaskId,
    types::{Aggregator, BackgroundTask, Connector, EventTask, EventType, Executor, Strategy},
};

pub struct TradingOrchestratorBuilder<E> {
    connectors: Vec<TaskId>,
    aggregator: Option<TaskId>,
    strategies: Vec<TaskId>,
    executors: Vec<TaskId>,
    scheduler: Scheduler<E>,
}

impl<E: EventType + 'static + ToString> TradingOrchestratorBuilder<E> {
    pub fn new(event_bus: EventBus<E>) -> Self {
        Self {
            connectors: Vec::new(),
            aggregator: None,
            strategies: Vec::new(),
            executors: Vec::new(),
            scheduler: Scheduler::new(event_bus),
        }
    }

    pub fn with_connector<T: Connector + 'static>(mut self, connector: Arc<Mutex<T>>) -> Self {
        let background_task: Arc<Mutex<dyn BackgroundTask>> = connector;
        let id = self.scheduler.register_background_task(background_task);
        self.connectors.push(id);
        self
    }

    pub fn with_aggregator<T: Aggregator + 'static>(mut self, aggregator: Arc<Mutex<T>>) -> Self {
        let background_task: Arc<Mutex<dyn BackgroundTask>> = aggregator;
        let id = self.scheduler.register_background_task(background_task);
        self.aggregator = Some(id);
        self
    }

    pub fn with_strategy<T: Strategy<E> + 'static>(mut self, strategy: Arc<Mutex<T>>) -> Self {
        let event_task: Arc<Mutex<dyn EventTask<E>>> = strategy;
        let id = self.scheduler.register_event_task(event_task);
        self.strategies.push(id);
        self
    }

    pub fn with_executor<T: Executor<E> + 'static>(mut self, executor: Arc<Mutex<T>>) -> Self {
        let event_task: Arc<Mutex<dyn EventTask<E>>> = executor;
        let id = self.scheduler.register_event_task(event_task);
        self.executors.push(id);
        self
    }

    pub fn build(self) -> Result<TradingOrchestrator<E>, OrchestratorError> {
        if self.connectors.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one connector is required",
            ));
        }
        if self.aggregator.is_none() {
            return Err(OrchestratorError::MissingComponent("aggregator"));
        }
        if self.strategies.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one strategy is required",
            ));
        }
        if self.executors.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one executor is required",
            ));
        }

        Ok(TradingOrchestrator {
            connectors: self.connectors,
            aggregator: self.aggregator.unwrap(),
            strategies: self.strategies,
            executors: self.executors,
            scheduler: self.scheduler,
        })
    }
}

pub struct TradingOrchestrator<E> {
    #[allow(dead_code)]
    connectors: Vec<TaskId>,
    #[allow(dead_code)]
    aggregator: TaskId,
    #[allow(dead_code)]
    strategies: Vec<TaskId>,
    #[allow(dead_code)]
    executors: Vec<TaskId>,
    scheduler: Scheduler<E>,
}

impl<E: EventType + 'static + ToString> TradingOrchestrator<E> {
    pub async fn start(&mut self) -> Result<(), OrchestratorError> {
        self.scheduler.start().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.scheduler.shutdown().await?;
        Ok(())
    }

    pub fn event_bus(&self) -> &EventBus<E> {
        self.scheduler.event_bus()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use crate::{
        channel_config::ChannelConfig,
        error::OrchestratorError,
        event_bus::EventBus,
        types::{Aggregator, BackgroundTask, Connector, EventTask, EventType, Executable, Executor, Strategy},
    };
    use super::*;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum TestEvent {
        MarketData,
        TradeSignal,
        Execution,
    }

    impl EventType for TestEvent {}

    impl ToString for TestEvent {
        fn to_string(&self) -> String {
            match self {
                TestEvent::MarketData => "MarketData".to_string(),
                TestEvent::TradeSignal => "TradeSignal".to_string(),
                TestEvent::Execution => "Execution".to_string(),
            }
        }
    }

    // Mock connector implementation
    struct MockConnector {
        name: String,
        initialized: bool,
        executed: bool,
        shutdown: bool,
    }

    impl MockConnector {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: false,
                executed: false,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockConnector {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl BackgroundTask for MockConnector {
        async fn execute(&mut self) -> Result<(), OrchestratorError> {
            self.executed = true;
            Ok(())
        }
    }

    #[async_trait]
    impl Connector for MockConnector {}

    // Mock aggregator implementation
    struct MockAggregator {
        name: String,
        initialized: bool,
        executed: bool,
        shutdown: bool,
    }

    impl MockAggregator {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: false,
                executed: false,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockAggregator {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl BackgroundTask for MockAggregator {
        async fn execute(&mut self) -> Result<(), OrchestratorError> {
            self.executed = true;
            Ok(())
        }
    }

    #[async_trait]
    impl Aggregator for MockAggregator {}

    // Mock strategy implementation
    struct MockStrategy {
        name: String,
        event: TestEvent,
        initialized: bool,
        handled_event: Option<String>,
        shutdown: bool,
    }

    impl MockStrategy {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                event: TestEvent::MarketData,
                initialized: false,
                handled_event: None,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockStrategy {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl EventTask<TestEvent> for MockStrategy {
        fn subscribed_event(&self) -> &TestEvent {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Strategy<TestEvent> for MockStrategy {}

    // Mock executor implementation
    struct MockExecutor {
        name: String,
        event: TestEvent,
        initialized: bool,
        handled_event: Option<String>,
        shutdown: bool,
    }

    impl MockExecutor {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                event: TestEvent::TradeSignal,
                initialized: false,
                handled_event: None,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockExecutor {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl EventTask<TestEvent> for MockExecutor {
        fn subscribed_event(&self) -> &TestEvent {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Executor<TestEvent> for MockExecutor {}

    fn create_test_event_bus() -> EventBus<TestEvent> {
        let configs = vec![
            (TestEvent::MarketData, ChannelConfig::new(10, "Market Data Channel".to_string())),
            (TestEvent::TradeSignal, ChannelConfig::new(10, "Trade Signal Channel".to_string())),
            (TestEvent::Execution, ChannelConfig::new(10, "Execution Channel".to_string())),
        ];
        EventBus::new(configs)
    }

    #[tokio::test]
    async fn test_trading_orchestrator_builder() {
        let event_bus = create_test_event_bus();
        
        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let aggregator = Arc::new(Mutex::new(MockAggregator::new("Aggregator")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));
        
        // Build the orchestrator
        let orchestrator = TradingOrchestratorBuilder::new(event_bus)
            .with_connector(connector.clone())
            .with_aggregator(aggregator.clone())
            .with_strategy(strategy.clone())
            .with_executor(executor.clone())
            .build()
            .expect("Failed to build orchestrator");
        
        // Verify the orchestrator was built correctly
        assert_eq!(orchestrator.connectors.len(), 1);
        assert_eq!(orchestrator.strategies.len(), 1);
        assert_eq!(orchestrator.executors.len(), 1);
        assert_eq!(orchestrator.event_bus().channel_count(), 3);
    }
    
    #[tokio::test]
    async fn test_orchestrator_missing_components() {
        let event_bus = create_test_event_bus();
        
        // Test missing connector
        let result = TradingOrchestratorBuilder::new(event_bus.clone())
            .with_aggregator(Arc::new(Mutex::new(MockAggregator::new("Aggregator"))))
            .with_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1"))))
            .with_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1"))))
            .build();
        
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("connector"));
        } else {
            panic!("Expected MissingComponent error for connector");
        }
        
        // Test missing aggregator
        let result = TradingOrchestratorBuilder::new(event_bus.clone())
            .with_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))))
            .with_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1"))))
            .with_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1"))))
            .build();
        
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert_eq!(component, "aggregator");
        } else {
            panic!("Expected MissingComponent error for aggregator");
        }
        
        // Test missing strategy
        let result = TradingOrchestratorBuilder::new(event_bus.clone())
            .with_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))))
            .with_aggregator(Arc::new(Mutex::new(MockAggregator::new("Aggregator"))))
            .with_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1"))))
            .build();
        
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("strategy"));
        } else {
            panic!("Expected MissingComponent error for strategy");
        }
        
        // Test missing executor
        let result = TradingOrchestratorBuilder::new(event_bus.clone())
            .with_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))))
            .with_aggregator(Arc::new(Mutex::new(MockAggregator::new("Aggregator"))))
            .with_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1"))))
            .build();
        
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("executor"));
        } else {
            panic!("Expected MissingComponent error for executor");
        }
    }
    
    #[tokio::test]
    async fn test_orchestrator_start_shutdown() {
        let event_bus = create_test_event_bus();
        
        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let aggregator = Arc::new(Mutex::new(MockAggregator::new("Aggregator")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));
        
        // Build the orchestrator
        let mut orchestrator = TradingOrchestratorBuilder::new(event_bus)
            .with_connector(connector.clone())
            .with_aggregator(aggregator.clone())
            .with_strategy(strategy.clone())
            .with_executor(executor.clone())
            .build()
            .expect("Failed to build orchestrator");
        
        // Start the orchestrator
        orchestrator.start().await.expect("Failed to start orchestrator");
        
        // Give some time for tasks to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Verify components were initialized
        assert!(connector.lock().await.initialized);
        assert!(aggregator.lock().await.initialized);
        assert!(strategy.lock().await.initialized);
        assert!(executor.lock().await.initialized);
        
        // Publish some events
        let market_data_sender = orchestrator.event_bus().clone_sender(&TestEvent::MarketData).unwrap();
        let _ = market_data_sender.send("market_data_event".to_string());
        
        let trade_signal_sender = orchestrator.event_bus().clone_sender(&TestEvent::TradeSignal).unwrap();
        let _ = trade_signal_sender.send("trade_signal_event".to_string());
        
        // Give some time for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Shutdown the orchestrator
        orchestrator.shutdown().await.expect("Failed to shutdown orchestrator");
        
        // Verify components were shutdown
        assert!(connector.lock().await.shutdown);
        assert!(aggregator.lock().await.shutdown);
        assert!(strategy.lock().await.shutdown);
        assert!(executor.lock().await.shutdown);
        
        // Verify background tasks were executed
        assert!(connector.lock().await.executed);
        assert!(aggregator.lock().await.executed);
        
        // Verify events were handled
        let strategy_lock = strategy.lock().await;
        assert!(strategy_lock.handled_event.is_some());
        if let Some(event_data) = &strategy_lock.handled_event {
            assert_eq!(event_data, "market_data_event");
        }
        
        let executor_lock = executor.lock().await;
        assert!(executor_lock.handled_event.is_some());
        if let Some(event_data) = &executor_lock.handled_event {
            assert_eq!(event_data, "trade_signal_event");
        }
    }
}
