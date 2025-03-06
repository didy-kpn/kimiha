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
    connectors: Vec<TaskId>,
    aggregator: TaskId,
    strategies: Vec<TaskId>,
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
