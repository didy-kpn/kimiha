use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    aggregator::Aggregator,
    channel_config::ChannelConfig,
    error::OrchestratorError,
    event_bus::EventBus,
    scheduler::Scheduler,
    task_id::TaskId,
    types::{BackgroundTask, Connector, EventTask, EventType, Executor, Strategy},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TradingEventType {
    MetricsUpdated,
    SignalGenerated,
}

impl EventType for TradingEventType {}

impl std::fmt::Display for TradingEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradingEventType::MetricsUpdated => write!(f, "METRICS_UPDATED"),
            TradingEventType::SignalGenerated => write!(f, "SIGNAL_GENERATED"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradingMessage {
    ExecuteArgs,
    None,
}

pub struct TradingOrchestrator {
    pub(crate) connector_ids: Vec<TaskId>,
    #[allow(dead_code)]
    pub(crate) aggregator_id: TaskId,
    pub(crate) strategy_ids: Vec<TaskId>,
    pub(crate) executor_ids: Vec<TaskId>,
    scheduler: Scheduler<TradingEventType, TradingMessage>,
}

impl Default for TradingOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl TradingOrchestrator {
    pub fn new() -> Self {
        let configs = vec![
            (
                TradingEventType::MetricsUpdated,
                ChannelConfig::new(10, "Metrics Updated Channel".to_string()),
            ),
            (
                TradingEventType::SignalGenerated,
                ChannelConfig::new(10, "Signal Generated Channel".to_string()),
            ),
        ];

        let event_bus = EventBus::<TradingEventType, TradingMessage>::new(configs);
        let mut scheduler = Scheduler::new(event_bus.clone());
        let event_sender = event_bus
            .clone_sender(&TradingEventType::MetricsUpdated)
            .expect("Failed to clone sender for MetricsUpdated");
        let aggregator = Aggregator::new("InternalAggregator".to_string(), event_sender);
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

    pub fn add_strategy<T: Strategy<TradingEventType, TradingMessage> + 'static>(
        &mut self,
        strategy: Arc<Mutex<T>>,
    ) {
        let task: Arc<Mutex<dyn EventTask<TradingEventType, TradingMessage>>> = strategy;
        let id = self.scheduler.register_event_task(task);
        self.strategy_ids.push(id);
    }

    pub fn add_executor<T: Executor<TradingEventType, TradingMessage> + 'static>(
        &mut self,
        executor: Arc<Mutex<T>>,
    ) {
        let task: Arc<Mutex<dyn EventTask<TradingEventType, TradingMessage>>> = executor;
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

        for strategy_id in &self.strategy_ids {
            let task = self
                .scheduler
                .task_registry()
                .get_event_task(strategy_id)
                .ok_or_else(|| OrchestratorError::TaskNotFound(strategy_id.clone()))?;
            let guard = task.lock().await;
            let event = guard.subscribed_event();
            if *event != TradingEventType::MetricsUpdated {
                return Err(OrchestratorError::InvalidTaskEventType(
                    "All strategy tasks must subscribe to TradingEventType::MetricsUpdated",
                ));
            }
        }

        for executor_id in &self.executor_ids {
            let task = self
                .scheduler
                .task_registry()
                .get_event_task(executor_id)
                .ok_or_else(|| OrchestratorError::TaskNotFound(executor_id.clone()))?;
            let guard = task.lock().await;
            let event = guard.subscribed_event();
            if *event != TradingEventType::SignalGenerated {
                return Err(OrchestratorError::InvalidTaskEventType(
                    "All executor tasks must subscribe to TradingEventType::SignalGenerated",
                ));
            }
        }

        self.scheduler.start().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        self.scheduler.shutdown().await?;
        Ok(())
    }

    pub fn event_bus(&self) -> &EventBus<TradingEventType, TradingMessage> {
        self.scheduler.event_bus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::OrchestratorError,
        types::{BackgroundTask, Connector, EventTask, Executable, Executor, Strategy},
    };
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

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
        event: TradingEventType,
        initialized: bool,
        handled_event: Option<TradingMessage>,
        shutdown: bool,
    }

    impl MockStrategy {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                event: TradingEventType::MetricsUpdated,
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
    impl EventTask<TradingEventType, TradingMessage> for MockStrategy {
        fn subscribed_event(&self) -> &TradingEventType {
            &self.event
        }

        async fn handle_event(&mut self, event: TradingMessage) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Strategy<TradingEventType, TradingMessage> for MockStrategy {}

    // Mock executor implementation
    struct MockExecutor {
        name: String,
        event: TradingEventType,
        initialized: bool,
        handled_event: Option<TradingMessage>,
        shutdown: bool,
    }

    impl MockExecutor {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                event: TradingEventType::SignalGenerated,
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
    impl EventTask<TradingEventType, TradingMessage> for MockExecutor {
        fn subscribed_event(&self) -> &TradingEventType {
            &self.event
        }

        async fn handle_event(&mut self, event: TradingMessage) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Executor<TradingEventType, TradingMessage> for MockExecutor {}

    #[tokio::test]
    async fn test_trading_orchestrator() {
        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));

        // Build the orchestrator using the internally created EventBus
        let mut orchestrator = TradingOrchestrator::new();
        orchestrator.add_connector(connector.clone());
        orchestrator.add_strategy(strategy.clone());
        orchestrator.add_executor(executor.clone());

        // Verify the orchestrator was built correctly
        assert_eq!(orchestrator.connector_ids.len(), 1);
        assert_eq!(orchestrator.strategy_ids.len(), 1);
        assert_eq!(orchestrator.executor_ids.len(), 1);
        // Now, the EventBus should have 2 channels: METRICS_UPDATED and SIGNAL_GENERATED
        assert_eq!(orchestrator.event_bus().channel_count(), 2);
    }

    #[tokio::test]
    async fn test_orchestrator_missing_components() {
        // Test missing connector
        let mut orchestrator = TradingOrchestrator::new();
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
        let mut orchestrator = TradingOrchestrator::new();
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
        let mut orchestrator = TradingOrchestrator::new();
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
        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));

        // Build the orchestrator
        let mut orchestrator = TradingOrchestrator::new();
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

        // Verify components were initialized
        assert!(connector.lock().await.initialized);
        assert!(strategy.lock().await.initialized);
        assert!(executor.lock().await.initialized);

        // Manually publish events
        let metrics_sender = orchestrator
            .event_bus()
            .clone_sender(&TradingEventType::MetricsUpdated)
            .unwrap();
        let _ = metrics_sender.send(TradingMessage::ExecuteArgs);

        let signal_sender = orchestrator
            .event_bus()
            .clone_sender(&TradingEventType::SignalGenerated)
            .unwrap();
        let _ = signal_sender.send(TradingMessage::ExecuteArgs);

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

        // Verify events were handled for strategy
        let strategy_lock = strategy.lock().await;
        assert!(strategy_lock.handled_event.is_some());
        if let Some(event_data) = &strategy_lock.handled_event {
            match event_data {
                TradingMessage::ExecuteArgs => {
                    panic!("Received TradingMessage::ExecuteArgs in strategy")
                }
                TradingMessage::None => {}
            }
        }

        // Verify events were handled for executor
        let executor_lock = executor.lock().await;
        assert!(executor_lock.handled_event.is_some());
        if let Some(event_data) = &executor_lock.handled_event {
            match event_data {
                TradingMessage::ExecuteArgs => {}
                TradingMessage::None => panic!("Received TradingMessage::None in executor"),
            }
        }
    }
}
