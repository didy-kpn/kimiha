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

pub struct TradingOrchestrator<M> {
    pub(crate) connector_ids: Vec<TaskId>,
    #[allow(dead_code)]
    pub(crate) aggregator_id: TaskId,
    pub(crate) strategy_ids: Vec<TaskId>,
    pub(crate) executor_ids: Vec<TaskId>,
    scheduler: Scheduler<TradingEventType, M>,
}

impl<M: Clone + Send + 'static + From<String>> Default for TradingOrchestrator<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + Send + 'static + From<String>> TradingOrchestrator<M> {
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

        let event_bus = EventBus::new(configs);
        let mut scheduler = Scheduler::new(event_bus.clone());
        let aggregator = Aggregator::new(
            "InternalAggregator".to_string(),
            event_bus.clone(),
            TradingEventType::MetricsUpdated,
        );
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

    // async版に変更: strategyのsubscribed_eventがMetricsUpdatedであるか検証します
    pub async fn add_strategy<T: Strategy<TradingEventType, M> + 'static>(
        &mut self,
        strategy: Arc<Mutex<T>>,
    ) -> Result<(), OrchestratorError> {
        {
            let guard = strategy.lock().await;
            if *guard.subscribed_event() != TradingEventType::MetricsUpdated {
                return Err(OrchestratorError::InvalidSubscribedEvent(
                    "Strategy subscribed event must be MetricsUpdated".to_string(),
                ));
            }
        }
        let task: Arc<Mutex<dyn EventTask<TradingEventType, M>>> = strategy;
        let id = self.scheduler.register_event_task(task);
        self.strategy_ids.push(id);
        Ok(())
    }

    // async版に変更: executorのsubscribed_eventがSignalGeneratedであるか検証します
    pub async fn add_executor<T: Executor<TradingEventType, M> + 'static>(
        &mut self,
        executor: Arc<Mutex<T>>,
    ) -> Result<(), OrchestratorError> {
        {
            let guard = executor.lock().await;
            if *guard.subscribed_event() != TradingEventType::SignalGenerated {
                return Err(OrchestratorError::InvalidSubscribedEvent(
                    "Executor subscribed event must be SignalGenerated".to_string(),
                ));
            }
        }
        let task: Arc<Mutex<dyn EventTask<TradingEventType, M>>> = executor;
        let id = self.scheduler.register_event_task(task);
        self.executor_ids.push(id);
        Ok(())
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

    pub fn event_bus(&self) -> &EventBus<TradingEventType, M> {
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
        handled_event: Option<String>,
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
    impl EventTask<TradingEventType, String> for MockStrategy {
        fn subscribed_event(&self) -> &TradingEventType {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Strategy<TradingEventType, String> for MockStrategy {}

    // Mock executor implementation
    struct MockExecutor {
        name: String,
        event: TradingEventType,
        initialized: bool,
        handled_event: Option<String>,
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
    impl EventTask<TradingEventType, String> for MockExecutor {
        fn subscribed_event(&self) -> &TradingEventType {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    #[async_trait]
    impl Executor<TradingEventType, String> for MockExecutor {}

    #[tokio::test]
    async fn test_trading_orchestrator() {
        // Create the components
        let connector = Arc::new(Mutex::new(MockConnector::new("Connector 1")));
        let strategy = Arc::new(Mutex::new(MockStrategy::new("Strategy 1")));
        let executor = Arc::new(Mutex::new(MockExecutor::new("Executor 1")));

        // Build the orchestrator using the internally created EventBus
        let mut orchestrator = TradingOrchestrator::<String>::new();
        orchestrator.add_connector(connector.clone());
        orchestrator.add_strategy(strategy.clone()).await.unwrap();
        orchestrator.add_executor(executor.clone()).await.unwrap();

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
        let mut orchestrator = TradingOrchestrator::<String>::new();
        orchestrator.add_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1")))).await.unwrap();
        orchestrator.add_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1")))).await.unwrap();

        let result = orchestrator.start().await;
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("connector"));
        } else {
            panic!("Expected MissingComponent error for connector");
        }

        // Test missing strategy
        let mut orchestrator = TradingOrchestrator::<String>::new();
        orchestrator.add_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))));
        orchestrator.add_executor(Arc::new(Mutex::new(MockExecutor::new("Executor 1")))).await.unwrap();

        let result = orchestrator.start().await;
        assert!(result.is_err());
        if let Err(OrchestratorError::MissingComponent(component)) = result {
            assert!(component.contains("strategy"));
        } else {
            panic!("Expected MissingComponent error for strategy");
        }

        // Test missing executor
        let mut orchestrator = TradingOrchestrator::<String>::new();
        orchestrator.add_connector(Arc::new(Mutex::new(MockConnector::new("Connector 1"))));
        orchestrator.add_strategy(Arc::new(Mutex::new(MockStrategy::new("Strategy 1")))).await.unwrap();

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
        let mut orchestrator = TradingOrchestrator::<String>::new();
        orchestrator.add_connector(connector.clone());
        orchestrator.add_strategy(strategy.clone()).await.unwrap();
        orchestrator.add_executor(executor.clone()).await.unwrap();

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
        let _ = metrics_sender.send("metrics_event".to_string());

        let signal_sender = orchestrator
            .event_bus()
            .clone_sender(&TradingEventType::SignalGenerated)
            .unwrap();
        let _ = signal_sender.send("signal_event".to_string());

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
            let is_manual_event = event_data == "metrics_event";
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
            assert_eq!(event_data, "signal_event");
        }
    }
}
