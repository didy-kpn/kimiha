use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    aggregator::Aggregator,
    error::OrchestratorError,
    event_bus::EventBus,
    scheduler::Scheduler,
    task_id::TaskId,
    types::{BackgroundTask, Connector, EventTask, EventType, Executor, Strategy},
};

pub struct TradingOrchestrator<E, M> {
    connector_ids: Vec<TaskId>,
    #[allow(dead_code)]
    aggregator_id: TaskId,
    strategy_ids: Vec<TaskId>,
    executor_ids: Vec<TaskId>,
    scheduler: Scheduler<E, M>,
}

impl<E: EventType + 'static + ToString, M: Clone + Send + 'static + From<String>>
    TradingOrchestrator<E, M>
{
    pub fn new(event_bus: EventBus<E, M>, market_event: E) -> Self {
        let mut scheduler = Scheduler::new(event_bus.clone());

        let aggregator_name = "InternalAggregator".to_string();
        let aggregator = Aggregator::new(aggregator_name, event_bus.clone(), market_event.clone());
        let aggregator_arc: Arc<Mutex<dyn BackgroundTask>> = Arc::new(Mutex::new(aggregator));
        let aggregator_id = scheduler.register_background_task(aggregator_arc);

        Self {
            connector_ids: Vec::new(),
            aggregator_id,
            strategy_ids: Vec::new(),
            executor_ids: Vec::new(),
            scheduler,
        }
    }

    pub fn add_connector<T: Connector + 'static>(&mut self, connector: Arc<Mutex<T>>) {
        let task: Arc<Mutex<dyn BackgroundTask>> = connector;
        let id = self.scheduler.register_background_task(task);
        self.connector_ids.push(id);
    }

    pub fn add_strategy<T: Strategy<E, M> + 'static>(&mut self, strategy: Arc<Mutex<T>>) {
        let task: Arc<Mutex<dyn EventTask<E, M>>> = strategy;
        let id = self.scheduler.register_event_task(task);
        self.strategy_ids.push(id);
    }

    pub fn add_executor<T: Executor<E, M> + 'static>(&mut self, executor: Arc<Mutex<T>>) {
        let task: Arc<Mutex<dyn EventTask<E, M>>> = executor;
        let id = self.scheduler.register_event_task(task);
        self.executor_ids.push(id);
    }

    pub async fn start(&mut self) -> Result<(), OrchestratorError> {
        if self.connector_ids.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one connector is required",
            ));
        }
        if self.strategy_ids.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one strategy is required",
            ));
        }
        if self.executor_ids.is_empty() {
            return Err(OrchestratorError::MissingComponent(
                "at least one executor is required",
            ));
        }
        self.scheduler.start().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.scheduler.shutdown().await?;
        Ok(())
    }

    pub fn event_bus(&self) -> &EventBus<E, M> {
        self.scheduler.event_bus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        channel_config::ChannelConfig,
        error::OrchestratorError,
        event_bus::EventBus,
        types::{BackgroundTask, Connector, EventTask, EventType, Executable, Executor, Strategy},
    };
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum TestEvent {
        MarketData,
        TradeSignal,
        Execution,
    }

    impl EventType for TestEvent {}

    impl std::fmt::Display for TestEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestEvent::MarketData => write!(f, "MarketData"),
                TestEvent::TradeSignal => write!(f, "TradeSignal"),
                TestEvent::Execution => write!(f, "Execution"),
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
    impl EventTask<TestEvent, String> for MockStrategy {
        fn subscribed_event(&self) -> &TestEvent {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Strategy<TestEvent, String> for MockStrategy {}

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
    impl EventTask<TestEvent, String> for MockExecutor {
        fn subscribed_event(&self) -> &TestEvent {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Executor<TestEvent, String> for MockExecutor {}

    fn create_test_event_bus() -> EventBus<TestEvent, String> {
        let configs = vec![
            (
                TestEvent::MarketData,
                ChannelConfig::new(10, "Market Data Channel".to_string()),
            ),
            (
                TestEvent::TradeSignal,
                ChannelConfig::new(10, "Trade Signal Channel".to_string()),
            ),
            (
                TestEvent::Execution,
                ChannelConfig::new(10, "Execution Channel".to_string()),
            ),
        ];
        EventBus::new(configs)
    }

    #[tokio::test]
    async fn test_trading_orchestrator() {
        let event_bus = create_test_event_bus();

        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));

        // Build the orchestrator
        let mut orchestrator = TradingOrchestrator::new(event_bus, TestEvent::MarketData);
        orchestrator.add_connector(connector.clone());
        orchestrator.add_strategy(strategy.clone());
        orchestrator.add_executor(executor.clone());

        // Verify the orchestrator was built correctly
        assert_eq!(orchestrator.connector_ids.len(), 1);
        assert_eq!(orchestrator.strategy_ids.len(), 1);
        assert_eq!(orchestrator.executor_ids.len(), 1);
        assert_eq!(orchestrator.event_bus().channel_count(), 3);
    }

    #[tokio::test]
    async fn test_orchestrator_missing_components() {
        let event_bus = create_test_event_bus();

        // Test missing connector
        let mut orchestrator = TradingOrchestrator::new(event_bus.clone(), TestEvent::MarketData);
        orchestrator.add_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1"))));
        orchestrator.add_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1"))));

        let result = orchestrator.start().await;
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("connector"));
        } else {
            panic!("Expected MissingComponent error for connector");
        }

        // Test missing strategy
        let mut orchestrator = TradingOrchestrator::new(event_bus.clone(), TestEvent::MarketData);
        orchestrator.add_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))));
        orchestrator.add_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1"))));

        let result = orchestrator.start().await;
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("strategy"));
        } else {
            panic!("Expected MissingComponent error for strategy");
        }

        // Test missing executor
        let mut orchestrator = TradingOrchestrator::new(event_bus.clone(), TestEvent::MarketData);
        orchestrator.add_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))));
        orchestrator.add_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1"))));

        let result = orchestrator.start().await;
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
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));

        // Build the orchestrator
        let mut orchestrator = TradingOrchestrator::new(event_bus, TestEvent::MarketData);
        orchestrator.add_connector(connector.clone());
        orchestrator.add_strategy(strategy.clone());
        orchestrator.add_executor(executor.clone());

        // Start the orchestrator
        orchestrator
            .start()
            .await
            .expect("Failed to start orchestrator");

        // Give some time for tasks to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify connector was initialized
        assert!(connector.lock().await.initialized);
        assert!(strategy.lock().await.initialized);
        assert!(executor.lock().await.initialized);

        // Manually publish events instead of relying on aggregator's events
        let market_data_sender = orchestrator
            .event_bus()
            .clone_sender(&TestEvent::MarketData)
            .unwrap();
        let _ = market_data_sender.send("market_data_event".to_string());

        let trade_signal_sender = orchestrator
            .event_bus()
            .clone_sender(&TestEvent::TradeSignal)
            .unwrap();
        let _ = trade_signal_sender.send("trade_signal_event".to_string());

        // Give some time for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Shutdown the orchestrator
        orchestrator
            .shutdown()
            .await
            .expect("Failed to shutdown orchestrator");

        // Verify components were shutdown
        assert!(connector.lock().await.shutdown);
        assert!(strategy.lock().await.shutdown);
        assert!(executor.lock().await.shutdown);

        // Verify background tasks were executed
        assert!(connector.lock().await.executed);

        // Verify events were handled
        let strategy_lock = strategy.lock().await;
        assert!(strategy_lock.handled_event.is_some());
        if let Some(event_data) = &strategy_lock.handled_event {
            let is_manual_event = event_data == "market_data_event";
            let is_json_snapshot = event_data.starts_with("{") && event_data.ends_with("}");
            assert!(
                is_manual_event || is_json_snapshot,
                "Event wasn't recognized: {}",
                event_data
            );
        }

        let executor_lock = executor.lock().await;
        assert!(executor_lock.handled_event.is_some());
        if let Some(event_data) = &executor_lock.handled_event {
            assert_eq!(event_data, "trade_signal_event");
        }
    }
}
